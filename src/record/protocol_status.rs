use crate::error::Error;
use std::io::{self, Write};

/// An indication of the completion status of a FastCGI request  
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolStatus {
    /// Normal end of request.
    RequestComplete,
    /// This happens when a FastCGI client sends concurrent requests over one
    /// connection to a FastCGI server that is designed to process one request at a time per
    /// connection.
    MultiplexingUnsupported,
    /// This happens when the FastCGI server runs out of some resource, e.g. database connections.
    Overloaded,
    /// This happens when the FastCGI server has specified a role that is unknown to the application.
    UnknownRole,
}

impl ProtocolStatus {
    pub(super) fn id(&self) -> u8 {
        match self {
            Self::RequestComplete => 0,
            Self::MultiplexingUnsupported => 1,
            Self::Overloaded => 2,
            Self::UnknownRole => 3,
        }
    }

    pub(super) fn from_record_byte(byte: u8) -> Result<Self, Error> {
        let status = match byte {
            0 => Self::RequestComplete,
            1 => Self::MultiplexingUnsupported,
            2 => Self::Overloaded,
            3 => Self::UnknownRole,
            _ => return Err(Error::UnspportedProtocolStatus(byte)),
        };

        Ok(status)
    }

    pub(super) fn as_record_byte<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        let id = self.id();
        writer.write_all(&[id])
    }
}
