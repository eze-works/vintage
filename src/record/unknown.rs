use crate::error::Error;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownType(u8);

impl UnknownType {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 8] = bytes
            .try_into()
            .map_err(|_| Error::MalformedRecordPayload("UnknownType"))?;

        Ok(Self(buffer[0]))
    }

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&[self.0, 0, 0, 0, 0, 0, 0, 0])
    }
}
