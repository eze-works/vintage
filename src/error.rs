use std::fmt::Display;
use std::io;

#[derive(Debug)]
pub enum Error {
    UnexpectedSocketClose(io::Error),
    UnsuportedVersion(u8),
    UnknownRecordType(u8),
    MultiplexingUnsupported,
    MalformedRecordPayload(&'static str),
    UnsupportedRole(u16),
    UnspportedProtocolStatus(u8),
    InvalidUtf8KeyValuePair,
    MalformedRecordStream,
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnexpectedSocketClose(_) => {
                write!(f, "The connection socket was closed unexpectedly")
            }
            Self::UnsuportedVersion(v) => {
                write!(f, "Unsupported FastCGI version: '{v}'")
            }
            Self::UnknownRecordType(t) => {
                write!(f, "Unknown record type: '{t}'")
            }
            Self::MultiplexingUnsupported => {
                write!(
                    f,
                    "Multiplexing multiple requests unto a single connection is not supported"
                )
            }
            Self::MalformedRecordPayload(s) => {
                write!(f, "Received malformed FastCGI record for type '{s}'")
            }
            Self::UnsupportedRole(r) => write!(f, "Unsuported FastCGI role: '{r}'"),
            Self::UnspportedProtocolStatus(s) => {
                write!(f, "Unsupported FastCGI protocol status: '{s}'")
            }
            Self::InvalidUtf8KeyValuePair => {
                write!(f, "Detected invalid utf8 in a key-value pair")
            }
            Self::MalformedRecordStream => {
                write!(f, "Web server sent a malformed record stream")
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnexpectedSocketClose(e) => Some(e),
            _ => None,
        }
    }
}
