use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `FCGI_GET_VALUES_RESULT` record
///
/// This is sent by a FastCGI server in response to a request with a `GetValues` record.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct GetValuesResult {
    values: BTreeMap<String, String>,
}

impl GetValuesResult {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            values: pairs::from_record_bytes(bytes)?,
        })
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.values, writer)
    }

    pub fn add<K, V>(mut self, key: K, value: V) -> Self
    where
        K: std::fmt::Display,
        V: std::fmt::Display,
    {
        self.values.insert(key.to_string(), value.to_string());
        self
    }
}
