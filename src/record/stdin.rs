use crate::error::Error;
use std::io::{self, Write};

/// A FastCGI `FCGI_STDIN` record
///
/// A record used to send arbitrary data from the FastCGI client to the server.
/// For example, this is used to send the POST request payload
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Stdin(pub Vec<u8>);

impl Stdin {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }

    /// Takes ownership of the data, leaving an empty `Vec` in its place.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.0)
    }
}
