use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_UNKNOWN_TYPE` record
///
/// To provide its evolution (lol), the spec includes the FCGI_UNKNOWN_TYPE management record
/// that can be used whenever a "non-recognized management record" is not received.
///
/// This begs the question: "What about non-recognized application records"? The says nothing about
/// this.
/// Therefore, this library also emits this record type in response to unrecognized application records.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnknownType(u8);

impl UnknownType {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let buffer: [u8; 8] = bytes
            .try_into()
            .map_err(|_| Error::MalformedRecordPayload("UnknownType"))?;

        Ok(Self(buffer[0]))
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&[self.0, 0, 0, 0, 0, 0, 0, 0])
    }

    pub fn new(type_id: u8) -> Self {
        Self(type_id)
    }
}
