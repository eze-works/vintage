use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `GET_VALUES` record
///
// A FastCGI client can query specific variables within a FastCGI server using this record type.
// It is designed to allow querying an open-ended set of variables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetValues {
    names: BTreeMap<String, String>,
}

impl GetValues {
    pub(super) fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            names: pairs::from_record_bytes(bytes)?,
        })
    }

    pub(super) fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.names, writer)
    }

    /// Returns an iterator over the list of variables whose values have been requested
    pub fn get_variables(&self) -> impl Iterator<Item = &str> {
        self.names.keys().map(|k| k.as_str())
    }

    /// Creates a new `FCGI_GET_VALUES` record
    pub fn new<I, T>(variables: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let key_values = variables.into_iter().map(|n| (n.into(), String::new()));

        Self {
            names: BTreeMap::from_iter(key_values),
        }
    }
}
