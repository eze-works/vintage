use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `GET_VALUES` record
///
// A FastCGI client can query specific variables within a FastCGI server using this record type.
// It is designed to allow querying an open-ended set of variables.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct GetValues {
    names: BTreeMap<String, String>,
}

impl GetValues {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            names: pairs::from_record_bytes(bytes)?,
        })
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.names, writer)
    }

    pub fn get_variables(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(|k| k.as_str())
    }

    pub fn add(mut self, name: impl std::fmt::Display) -> Self {
        self.names.insert(name.to_string(), String::new());
        self
    }
}
