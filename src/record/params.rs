use std::collections::BTreeMap;
use super::pairs;
use crate::error::Error;
use std::io::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params(BTreeMap<String, String>);

impl Params {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(pairs::from_record_bytes(bytes)?))
    }

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.0, writer)
    }
}
