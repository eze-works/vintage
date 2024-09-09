use crate::error::Error;
use std::io::{self, Write};
use super::pairs;
use std::collections::BTreeMap;

// A FastCGI client can query specific variables within the FastCGI server via this request
// It is designed to allow an open-ended set of variables.
#[derive(Debug, Clone)]
pub struct GetValues {
    // From the spec:
    // "The contentData portion of a FCGI_GET_VALUES record contains a sequence of name-value pairs with empty values."
    names: BTreeMap<String, String> 
}

impl GetValues {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        Ok(Self {
            names: pairs::from_record_bytes(bytes)?
        })
    }

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        pairs::to_record_bytes(&self.names, writer)
    }
}


