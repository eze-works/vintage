use crate::error::Error;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AbortRequest;

impl AbortRequest {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        // This record type has no body
        if !bytes.is_empty() {
            return Err(Error::MalformedRecordPayload("AbortRequest"));
        }
        Ok(AbortRequest)
    }

    pub fn to_record_bytes<W: Write>(&self, _writer: &mut W) -> Result<(), io::Error> {
        Ok(())
    }
}
