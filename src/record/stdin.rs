use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_STDIN` record
///
/// A record used to send arbitrary data from the FastCGI client to the server.
/// For example, this is used to send the POST request payload
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stdin(Vec<u8>);

impl Stdin {
    pub(super) fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub(super) fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }

    /// Creates a new `FCGI_STDIN` record
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}
