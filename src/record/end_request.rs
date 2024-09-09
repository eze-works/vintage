use super::protocol_status::ProtocolStatus;
use crate::error::Error;
use std::io::{self, Write};

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
            protocol_status
        })
    }


    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.exit_code.to_be_bytes())?;
        writer.write_all(&[self.protocol_status.id()])
    }
}
