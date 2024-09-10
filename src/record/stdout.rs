use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_STDOUT` record
///
/// Used to send arbitrary data from the FastCGI server to the client.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Stdout(Vec<u8>);

impl Stdout {
    pub(super) fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub(super) fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }

    /// Creates a new `FCGI_STDOUT` record
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

