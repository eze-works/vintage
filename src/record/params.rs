use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `FCGI_PARAMS` record
///
/// Used for sending name-value pairs between FastCGI server and client
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Params(BTreeMap<String, String>);

impl Params {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(pairs::from_record_bytes(bytes)?))
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.0, writer)
    }

    pub fn add<K, V>(mut self, key: K, value: V) -> Self
    where
        K: std::fmt::Display,
        V: std::fmt::Display,
    {
        self.0.insert(key.to_string(), value.to_string());
        self
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|s| s.as_str())
    }

    pub fn take(&mut self) -> BTreeMap<String, String> {
        std::mem::take(&mut self.0)
    }
}
