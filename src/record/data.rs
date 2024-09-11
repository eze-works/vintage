use crate::error::Error;
use std::io::{self, Write};

// A FastCGI `FCGI_DATA` record
//
// Similar to `FCGI_STDIN`, but does not get used as part of the `Responder` flow.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Data(pub Vec<u8>);

impl Data {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }

    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}
