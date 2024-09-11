use super::protocol_status::ProtocolStatus;
use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_END_REQUEST` record
///
/// This record is used by a FastCGI server to indicate a request is complete, either because it
/// has been processed successfully or because it has been rejected.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EndRequest {
    exit_code: u32,
    protocol_status: ProtocolStatus,
}

impl EndRequest {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 8] = bytes
            .try_into()
            .map_err(|_| Error::MalformedRecordPayload("EndRequest"))?;

        let exit_code = u32::from_be_bytes((&buffer[..4]).try_into().unwrap());
        let protocol_status = ProtocolStatus::from_record_byte(buffer[4])?;
        Ok(Self {
            exit_code,
            protocol_status,
        })
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.exit_code.to_be_bytes())?;
        self.protocol_status.as_record_byte(writer)?;
        writer.write_all(&[0, 0, 0])
    }

    pub fn new(exit_code: u32, status: ProtocolStatus) -> Self {
        Self {
            exit_code,
            protocol_status: status,
        }
    }
}
