use crate::error::Error;
use crate::record::{self, *};
use bufstream::BufStream;
use mio::net::{TcpStream, UnixStream};
#[cfg(test)]
use std::collections::VecDeque;
use std::io::{self, Read, Write};

#[derive(Debug)]
pub enum Connection {
    Tcp(BufStream<TcpStream>),
    UnixSocket(BufStream<UnixStream>),
    #[cfg(test)]
    Test(VecDeque<u8>),
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(w) => w.write(buf),
            Connection::UnixSocket(w) => w.write(buf),
            #[cfg(test)]
            Connection::Test(w) => w.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            Connection::Tcp(w) => w.flush(),
            Connection::UnixSocket(w) => w.flush(),
            #[cfg(test)]
            Connection::Test(w) => w.flush(),
        }
    }
}

impl Read for Connection {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            Connection::Tcp(r) => r.read(buf),
            Connection::UnixSocket(r) => r.read(buf),
            #[cfg(test)]
            Connection::Test(r) => r.read(buf),
        }
    }
}

impl From<TcpStream> for Connection {
    fn from(value: TcpStream) -> Self {
        Connection::Tcp(BufStream::new(value))
    }
}

impl From<UnixStream> for Connection {
    fn from(value: UnixStream) -> Self {
        Connection::UnixSocket(BufStream::new(value))
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

        Ok(Packet { type_id, content })
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

    pub fn write_record(&mut self, record: &Record) -> Result<(), io::Error> {
        // We need the payload length in order to figure out the length of the padding
        let mut payload = vec![];
        record.write_bytes(&mut payload)?;

        // Length of Header + Length of Payload
        let unpadded_len = 8 + payload.len();

        // Figure out the closest factor of 8 that is greater than the unpadded length
        let padded_len = unpadded_len.div_ceil(8) * 8;

        // The amount of padding is the difference between those numers
        let padding = (padded_len - unpadded_len) as u8;

        let request_id = if record.is_management_record() {
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
    //
    // Some records are "stream" records, so this function allows sending a sequence of records and
    // asserting that they come out on the "other side" stiched together into one record
    #[track_caller]
    fn round_trip<T: IntoIterator<Item = Record>>(send: T, receive: Record) {
        let mut connection = Connection::Test(VecDeque::new());

        for r in send.into_iter() {
            connection.write_record(&r).unwrap();
        }
        let from_client = connection.read_record().unwrap();
        assert_eq!(receive, from_client);
    }

    #[test]
    fn get_values() {
        round_trip(
            [Record::GetValues(GetValues::default())],
            Record::GetValues(GetValues::default()),
        );
        round_trip(
            [Record::GetValues(
                GetValues::default().add("FCGI_MAX_CONNS"),
            )],
            Record::GetValues(GetValues::default().add("FCGI_MAX_CONNS")),
        );
    }

    #[test]
    fn get_values_result() {
        round_trip(
            [Record::GetValuesResult(GetValuesResult::default())],
            Record::GetValuesResult(GetValuesResult::default()),
        );

        round_trip(
            [Record::GetValuesResult(
                GetValuesResult::default().add("FCGI_MAX_REQS", "1"),
            )],
            Record::GetValuesResult(GetValuesResult::default().add("FCGI_MAX_REQS", "1")),
        );
    }

    #[test]
    fn unknown_type() {
        round_trip(
            [Record::UnknownType(UnknownType::new(100))],
            Record::UnknownType(UnknownType::new(100)),
        );
    }

    #[test]
    fn begin_request() {
        round_trip(
            [Record::BeginRequest(BeginRequest::new(
                Role::Responder,
                true,
            ))],
            Record::BeginRequest(BeginRequest::new(Role::Responder, true)),
        );
    }

    #[test]
    fn params() {
        round_trip(
            [Record::Params(Params::default())],
            Record::Params(Params::default()),
        );

        round_trip(
            [
                Record::Params(Params::default().add("PATH", "/home")),
                Record::Params(Params::default()),
            ],
            Record::Params(Params::default().add("PATH", "/home")),
        );
    }

    #[test]
    fn stdin() {
        round_trip(
            [Record::Stdin(Stdin::new(vec![]))],
            Record::Stdin(Stdin::new(vec![])),
        );

        round_trip(
            [
                Record::Stdin(Stdin::new(b"HELLO".into())),
                Record::Stdin(Stdin::new(b"WORLD".into())),
                Record::Stdin(Stdin::new(vec![])),
            ],
            Record::Stdin(Stdin::new(b"HELLOWORLD".into())),
        );
    }

    #[test]
    fn stdout() {
        round_trip(
            [Record::Stdout(Stdout::new(vec![]))],
            Record::Stdout(Stdout::new(vec![])),
        );

        round_trip(
            [
                Record::Stdout(Stdout::new(b"HELLO".into())),
                Record::Stdout(Stdout::new(b"WORLD".into())),
                Record::Stdout(Stdout::new(vec![])),
            ],
            Record::Stdout(Stdout::new(b"HELLOWORLD".into())),
        );
    }

    #[test]
    fn stderr() {
        round_trip(
            [Record::Stderr(Stderr::new(vec![]))],
            Record::Stderr(Stderr::new(vec![])),
        );

        round_trip(
            [
                Record::Stderr(Stderr::new(b"HELLO".into())),
                Record::Stderr(Stderr::new(b"WORLD".into())),
                Record::Stderr(Stderr::new(vec![])),
            ],
            Record::Stderr(Stderr::new(b"HELLOWORLD".into())),
        );
    }

    #[test]
    fn data() {
        round_trip(
            [Record::Data(Data::new(vec![]))],
            Record::Data(Data::new(vec![])),
        );

        round_trip(
            [
                Record::Data(Data::new(b"HELLO".into())),
                Record::Data(Data::new(b"WORLD".into())),
                Record::Data(Data::new(vec![])),
            ],
            Record::Data(Data::new(b"HELLOWORLD".into())),
        );
    }

    #[test]
    fn abort_request() {
        round_trip(
            [Record::AbortRequest(AbortRequest)],
            Record::AbortRequest(AbortRequest),
        );
    }

    #[test]
    fn end_request() {
        round_trip(
            [Record::EndRequest(EndRequest::new(
                0,
                ProtocolStatus::RequestComplete,
            ))],
            Record::EndRequest(EndRequest::new(0, ProtocolStatus::RequestComplete)),
        );

        round_trip(
            [Record::EndRequest(EndRequest::new(
                1,
                ProtocolStatus::UnknownRole,
            ))],
            Record::EndRequest(EndRequest::new(1, ProtocolStatus::UnknownRole)),
        );
    }
}
