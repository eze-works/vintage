use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_STDERR` record
///
/// Used to send error data from the FastCGI server to the client.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stderr(Vec<u8>);

impl Stderr {
    pub(super) fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub(super) fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }

    /// Creates a new `FCGI_STDERR` record
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}
