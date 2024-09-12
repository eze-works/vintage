use crate::error::Error;
use crate::record::{self, *};
use bufstream::BufStream;
#[cfg(test)]
use std::collections::VecDeque;
use std::io::{self, Read, Write};
use std::net::TcpStream;

#[derive(Debug)]
pub enum Connection {
    Tcp(BufStream<TcpStream>),
    #[cfg(test)]
    Test(VecDeque<u8>),
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(w) => w.write(buf),
            #[cfg(test)]
            Connection::Test(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Connection::Tcp(w) => w.flush(),
            #[cfg(test)]
            Connection::Test(w) => w.flush(),
        }
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(r) => r.read(buf),
            #[cfg(test)]
            Connection::Test(r) => r.read(buf),
        }
    }
}

impl TryFrom<mio::net::TcpStream> for Connection {
    type Error = io::Error;

    fn try_from(value: mio::net::TcpStream) -> Result<Self, Self::Error> {
        // Convert to a regular blocking TcpStream here, since it would be annoying to manage a mio
        // event loop for every call to read/write/flush
        // Additionally add a timeout for io operations so that an idle connection is not kept open
        // indefinitely
        let stream = TcpStream::from(value);
        stream.set_nonblocking(false)?;
        let timeout = std::time::Duration::from_secs(3);
        stream.set_read_timeout(Some(timeout))?;
        Ok(Connection::Tcp(BufStream::new(stream)))
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
pub struct Packet {
    pub type_id: u8,
    pub content: Vec<u8>,
}

impl Packet {
    fn is_discrete(&self) -> bool {
        record::DISCRETE_RECORD_TYPES.contains(&self.type_id)
    }

    fn is_management_record(&self) -> bool {
        record::MANAGEMENT_RECORD_TYPES.contains(&self.type_id)
    }

    fn is_empty(&self) -> bool {
        self.content.is_empty()
    }
}

impl Connection {
    pub fn read_packet(&mut self) -> Result<Packet, Error> {
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

        Ok(Packet { type_id, content })
    }

    pub fn write_packet(&mut self, packet: &Packet) -> Result<(), io::Error> {
        let payload = &packet.content;

        // Length of Header + Length of Payload
        let unpadded_len = 8 + payload.len();

        // Figure out the closest factor of 8 that is greater than the unpadded length
        let padded_len = unpadded_len.div_ceil(8) * 8;

        // The amount of padding is the difference between those numers
        let padding = (padded_len - unpadded_len) as u8;

        let request_id = if packet.is_management_record() {
            [0, 0]
        } else {
            [0, 1]
        };

        // Version + Record type
        self.write_all(&[1, packet.type_id])?;
        // Request ID
        self.write_all(&request_id)?;
        // Payload length
        self.write_all(&(payload.len() as u16).to_be_bytes())?;
        // Padding length + Reserved field
        self.write_all(&[padding, 0])?;
        // Payload
        self.write_all(payload)?;
        // Padding
        self.write_all(&vec![0u8; padding as usize])?;
        // Don't forget to flush.
        self.flush()
    }

    pub fn read_record(&mut self) -> Result<Record, Error> {
        let first = self.read_packet()?;
        let expected_type_id = first.type_id;

        if first.is_discrete() || first.is_empty() {
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

    pub fn write_record(&mut self, record: &Record) -> Result<(), io::Error> {
        let mut payload = vec![];
        record.write_bytes(&mut payload)?;

        // The length of the payload must be able to fit in two bytes.
        let mut payload_chunks: Vec<Vec<_>> = payload
            .chunks(u16::MAX as usize)
            .map(<[u8]>::to_vec)
            .collect();

        // Always write an empty chunk.
        // + For stream records, this will be used to terminate the stream
        // + For empty discrete records, this will be the only chunk written
        payload_chunks.push(vec![]);

        for chunk in payload_chunks {
            let packet = Packet {
                type_id: record.type_id(),
                content: chunk,
            };
            self.write_packet(&packet)?;

            // Discrete records should always fit in a single packet. So write exactly one, and
            // break out
            if packet.is_discrete() {
                break;
            }
        }

        Ok(())
    }

    impl_expect!(GetValues);
    impl_expect!(GetValuesResult);
    impl_expect!(UnknownType);
    impl_expect!(BeginRequest);
    impl_expect!(EndRequest);
    impl_expect!(Params);
    impl_expect!(AbortRequest);
    impl_expect!(Stdin);
    impl_expect!(Stdout);
    impl_expect!(Stderr);
    impl_expect!(Data);
}

macro_rules! impl_expect {
    ($t:path) => {
        paste::paste! {
            #[doc =
                "Returns the next record if it is a [`" $t "`](crate::record::" $t ") record.\n\n"
                "# Errors\n\n"
                "Returns `Err(Some(Error))` if reading the connection failed.\n\n"
                "Returns `Err(None)` if the next record was something else"
            ]
            pub fn [<expect_ $t:snake>](&mut self) -> Result<$t, Option<Error>> {
                match self.read_record() {
                    Ok(Record::$t(r)) => Ok(r),
                    Ok(_) => Err(None),
                    Err(e) => Err(Some(e))
                }
            }
        }
    };
}
pub(crate) use impl_expect;

#[cfg(test)]
mod round_trip_tests {
    use super::*;

    // Test that records can be serialized and deserialized without loosing information.
    #[track_caller]
    fn round_trip(send: impl Into<Record>) {
        let mut connection = Connection::Test(VecDeque::new());

        let record = send.into();
        connection.write_record(&record).unwrap();

        let received = connection.read_record().unwrap();
        assert_eq!(received, record);
    }

    #[test]
    fn get_values() {
        round_trip(GetValues::default());
        round_trip(GetValues::default().add("FCGI_MAX_CONNS"));
    }

    #[test]
    fn get_values_result() {
        round_trip(GetValuesResult::default());

        round_trip(GetValuesResult::default().add("FCGI_MAX_REQS", "1"));
    }

    #[test]
    fn unknown_type() {
        round_trip(UnknownType(100));
    }

    #[test]
    fn begin_request() {
        round_trip(BeginRequest::new(Role::Responder, true));
    }

    #[test]
    fn params() {
        round_trip(Params::default());

        round_trip(Params::default().add("PATH", "/home"));
    }

    #[test]
    fn stdin() {
        round_trip(Stdin(vec![]));

        round_trip(Stdin(b"HELLO".into()));
    }

    #[test]
    fn stdout() {
        round_trip(Stdout(vec![]));

        round_trip(Stdout(b"HELLO".into()));
    }

    #[test]
    fn stderr() {
        round_trip(Stderr(vec![]));

        round_trip(Stderr(b"HELLO".into()));
    }

    #[test]
    fn data() {
        round_trip(Data(vec![]));

        round_trip(Data(b"HELLO".into()));
    }

    #[test]
    fn abort_request() {
        round_trip(AbortRequest);
    }

    #[test]
    fn end_request() {
        round_trip(EndRequest::new(0, ProtocolStatus::RequestComplete));

        round_trip(EndRequest::new(1, ProtocolStatus::UnknownRole));
    }
}

#[cfg(test)]
mod stream_parsing_tests {
    use super::*;
    use assert_matches::assert_matches;

    #[test]
    fn stream_packet_are_concatenated_when_read() {
        let mut connection = Connection::Test(VecDeque::new());

        let packets = [
            Packet {
                type_id: record::FCGI_STDOUT,
                content: b"HEL".to_vec(),
            },
            Packet {
                type_id: record::FCGI_STDOUT,
                content: b"LO".to_vec(),
            },
            Packet {
                type_id: record::FCGI_STDOUT,
                content: b"WORLD".to_vec(),
            },
            Packet {
                type_id: record::FCGI_STDOUT,
                content: vec![],
            },
        ];

        for packet in packets {
            connection.write_packet(&packet).unwrap();
        }

        let actual = connection.read_record().unwrap();
        let expected = Record::from(Stdout(b"HELLOWORLD".to_vec()));
        assert_eq!(actual, expected);
    }

    #[test]
    fn stream_packets_are_broken_up_when_written() {
        let mut connection = Connection::Test(VecDeque::new());
        let payload_length = u16::MAX as usize * 5;
        let payload = b"A".repeat(payload_length);

        let record = Record::from(Stdout(payload.clone()));
        connection.write_record(&record).unwrap();

        // The payload was larger than the length field in the FastCGI protocol.
        // This read won't work if the code naively just wrote everything as a single packet
        // because the resulting bytes won't conform to the spec anymore.
        let result = connection.read_record();

        assert_matches!(result, Ok(_));
        let result = result.unwrap();
        assert_eq!(result, Record::from(Stdout(payload)));
    }
}
