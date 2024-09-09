use std::io::{self, Write};
use crate::error::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stdout(Vec<u8>);

impl Stdout {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(bytes))
    }

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        writer.write_all(&self.0)
    }
}
