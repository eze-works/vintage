use super::pairs;
use crate::error::Error;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// A FastCGI `FCGI_PARAMS` record
///
/// Used for sending name-value pairs between FastCGI server and client
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Params(BTreeMap<String, String>);

impl Params {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self(pairs::from_record_bytes(bytes)?))
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.0, writer)
    }

    pub fn new<I, T>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (T, T)>,
        T: Into<String>,
    {
        let pairs = BTreeMap::from_iter(pairs.into_iter().map(|(n, v)| (n.into(), v.into())));
        Self(pairs)
    }

    pub fn get(&self, name: &str) -> Option<&str> {
        self.0.get(name).map(|s| s.as_str())
    }
}
