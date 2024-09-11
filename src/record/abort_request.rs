use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_ABORT_REQUEST` record
///
/// A FastCGI client may send this to abort an in-flight request.
///
/// This struct has no members, so constructing it is as simple as naming the type.
///
/// Note: This record is widely un-implemented in most FastCGI clients, so it is very very
/// unlikely to be ever received. Nevertheless, we are nothing if not thorough, so it is still
/// defined here.
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

    pub fn write_record_bytes<W: Write>(&self, _writer: &mut W) -> Result<(), io::Error> {
        Ok(())
    }
}
