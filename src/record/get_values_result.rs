use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `FCGI_GET_VALUES_RESULT` record
///
/// This is sent by a FastCGI server in response to a request with a
/// [`GetValues`](crate::record::GetValues) record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetValuesResult {
    values: BTreeMap<String, String>,
}

impl GetValuesResult {
    pub(super) fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            values: pairs::from_record_bytes(bytes)?,
        })
    }

    pub(super) fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.values, writer)
    }

    /// Creates a new `FCGI_GET_VALUES_RESULT` record
    pub fn new<I, T>(values: I) -> Self 
        where I: IntoIterator<Item = (T, T)>,
              T: Into<String>
    {
        let values = BTreeMap::from_iter(values.into_iter().map(|(n, v)| (n.into(), v.into())));
        Self { values }
    }
}
