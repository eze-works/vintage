use crate::error::Error;
use crate::record::{self, Record};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
#[cfg(target_family = "unix")]
use std::os::unix::net::UnixStream;

#[derive(Debug)]
pub enum Connection {
    Tcp(BufReader<TcpStream>, BufWriter<TcpStream>),
    #[cfg(target_family = "unix")]
    UnixSocket(BufReader<UnixStream>, BufWriter<UnixStream>),
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(_, w) => w.write(buf),
            Connection::UnixSocket(_, w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Connection::Tcp(_, w) => w.flush(),
            Connection::UnixSocket(_, w) => w.flush(),
        }
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(r, _) => r.read(buf),
            Connection::UnixSocket(r, _) => r.read(buf),
        }
    }
}

impl TryFrom<TcpStream> for Connection {
    type Error = Error;
    fn try_from(value: TcpStream) -> Result<Self, Self::Error> {
        let reader = value;
        let writer = reader
            .try_clone()
            .map_err(Error::Socket)?;
        Ok(Self::Tcp(BufReader::new(reader), BufWriter::new(writer)))
    }
}

#[cfg(target_family = "unix")]
impl TryFrom<UnixStream> for Connection {
    type Error = Error;
    fn try_from(value: UnixStream) -> Result<Self, Self::Error> {
        let reader = value;
        let writer = reader
            .try_clone()
            .map_err(Error::Socket)?;
        Ok(Self::UnixSocket(
            BufReader::new(reader),
            BufWriter::new(writer),
        ))
    }
}

// A FastCGI client may send content using one or more FastCGI records 
// If the payload is sent in one "record", well then that's a complete record.
// If it's sent over multiple "records", each of them is incomplete, and the FastCGI server (us)
// needs to put them together to get the complete payload.. which is also called a "record" by the
// spec.
//
// This naming is confusing so this code follows the following convention:
// + Packet: A single, and potentially incomplete physical message sent by a FastCGI client. 
// + Record: A logically complete FastCGI message. You might need multiple packets to assemble one.  
#[derive(Debug, Clone)]
struct Packet {
    type_id: u8,
    content: Vec<u8>,
}

impl Packet {
    fn is_incomplete(&self) -> bool {
        record::DISCRETE_RECORD_TYPES.contains(&self.type_id)
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl Connection {
    fn read_packet(&mut self) -> Result<Packet, Error> {
        let mut header = [0u8; 8];
        self.read_exact(&mut header)
            .map_err(Error::UnexpectedSocketClose)?;

        let [version, type_id, req_id_1, req_id_0, length_1, length_0, padding_length, _] = header;

        if version != 1 {
            return Err(Error::UnsuportedVersion(version));
        }

        let req_id = u16::from_be_bytes([req_id_1, req_id_0]);

        if req_id > 1 {
            return Err(Error::MultiplexingUnsupported);
        }

        let length = u16::from_be_bytes([length_1, length_0]);
        let mut content = vec![0u8; length as usize];
        let mut padding = vec![0u8; padding_length as usize];

        self.read_exact(&mut content)
            .map_err(Error::UnexpectedSocketClose)?;
        self.read_exact(&mut padding)
            .map_err(Error::UnexpectedSocketClose)?;

        Ok(Packet {
            type_id,
            content,
        })
    }

    pub fn read_record(&mut self) -> Result<Record, Error> {
        let first = self.read_packet()?;
        let expected_type_id = first.type_id;

        if first.is_incomplete() || first.is_empty() {
            let record = Record::from_bytes(expected_type_id, first.content)?;
            return Ok(record);
        }

        let mut packets = vec![first];

        loop {
            let packet = self.read_packet()?;

            if packet.type_id != expected_type_id {
                return Err(Error::MalformedRecordStream);
            }

            if packet.is_empty() {
                break;
            }
            packets.push(packet);
        }

        let content = packets
            .into_iter()
            .flat_map(|r| r.content)
            .collect::<Vec<_>>();

        let record = Record::from_bytes(expected_type_id, content)?;

        Ok(record)
    }

    pub fn write_record(&mut self, record: Record) -> Result<(), io::Error> {
        // We need the payload length in order to figure out the length of the padding
        let mut payload = vec![];
        record.to_bytes(&mut payload)?;

        // Length of Header + Length of Payload
        let unpadded_len = 8 + payload.len();

        // Figure out the closest factor of 8 that is greater than the unpadded length
        let padded_len = unpadded_len.div_ceil(8) * 8;

        // The amount of padding is the difference between those numers
        let padding = (padded_len - unpadded_len) as u8;

        let request_id = if record::MANAGEMENT_RECORD_TYPES.contains(&record.type_id()) {
            [0, 0]
        } else {
            [0, 1]
        };

        // Version + Record type
        self.write_all(&[1, record.type_id()])?;
        // Request ID (which is always 1)
        self.write_all(&request_id)?;
        // Payload length
        self.write_all(&(payload.len() as u16).to_be_bytes())?;
        // Padding length + Reserved field
        self.write_all(&[padding, 0])?;
        // Payload
        self.write_all(&payload)?;
        // Padding
        self.write_all(&vec![0u8; padding as usize])?;
        // Don't forget to flush.
        self.flush()
    }
}
