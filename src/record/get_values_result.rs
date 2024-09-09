use crate::error::Error;
use std::io::{self, Write};
use super::pairs;
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct GetValuesResult {
    values: BTreeMap<String, String>
}

impl GetValuesResult {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            values: pairs::from_record_bytes(bytes)?
        })
    }

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.values, writer)
    }
}
