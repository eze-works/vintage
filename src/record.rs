mod abort_request;
mod begin_request;
mod data;
mod end_request;
mod get_values;
mod get_values_result;
mod pairs;
mod params;
mod protocol_status;
mod role;
mod stderr;
mod stdin;
mod stdout;
mod unknown;

use crate::error::Error;
pub use abort_request::AbortRequest;
pub use begin_request::BeginRequest;
pub use data::Data;
pub use end_request::EndRequest;
pub use get_values::GetValues;
pub use get_values_result::GetValuesResult;
pub use params::Params;
pub use protocol_status::ProtocolStatus;
#[cfg(test)]
pub use role::Role;
use std::io::{self, Write};
pub use stderr::Stderr;
pub use stdin::Stdin;
pub use stdout::Stdout;
pub use unknown::UnknownType;

const FCGI_BEGIN_REQUEST: u8 = 1;
const FCGI_ABORT_REQUEST: u8 = 2;
const FCGI_END_REQUEST: u8 = 3;
const FCGI_PARAMS: u8 = 4;
const FCGI_STDIN: u8 = 5;
const FCGI_STDOUT: u8 = 6;
const FCGI_STDERR: u8 = 7;
const FCGI_DATA: u8 = 8;
const FCGI_GET_VALUES: u8 = 9;
const FCGI_GET_VALUES_RESULT: u8 = 10;
const FCGI_UNKNOWN_TYPE: u8 = 11;

pub(super) const DISCRETE_RECORD_TYPES: [u8; 6] = [
    FCGI_GET_VALUES,
    FCGI_GET_VALUES_RESULT,
    FCGI_UNKNOWN_TYPE,
    FCGI_BEGIN_REQUEST,
    FCGI_ABORT_REQUEST,
    FCGI_END_REQUEST,
];

pub(super) const MANAGEMENT_RECORD_TYPES: [u8; 3] =
    [FCGI_GET_VALUES, FCGI_GET_VALUES_RESULT, FCGI_UNKNOWN_TYPE];



/// A single FastCGI message
///
/// All data that flows between FastCGI client and server is carried in records. The variant used
/// communicates the intent of the message
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Record {
    GetValues(GetValues),
    GetValuesResult(GetValuesResult),
    BeginRequest(BeginRequest),
    Params(Params),
    Stdin(Stdin),
    Data(Data),
    Stdout(Stdout),
    Stderr(Stderr),
    AbortRequest(AbortRequest),
    UnknownType(UnknownType),
    EndRequest(EndRequest),
}

impl Record {
    pub fn type_id(&self) -> u8 {
        match self {
            Self::GetValues(_) => FCGI_GET_VALUES,
            Self::GetValuesResult(_) => FCGI_GET_VALUES_RESULT,
            Self::BeginRequest(_) => FCGI_BEGIN_REQUEST,
            Self::Params(_) => FCGI_PARAMS,
            Self::Stdin(_) => FCGI_STDIN,
            Self::Data(_) => FCGI_DATA,
            Self::Stdout(_) => FCGI_STDOUT,
            Self::Stderr(_) => FCGI_STDERR,
            Self::AbortRequest(_) => FCGI_ABORT_REQUEST,
            Self::EndRequest(_) => FCGI_END_REQUEST,
            Self::UnknownType(_) => FCGI_UNKNOWN_TYPE,
        }
    }

    pub fn from_bytes(type_id: u8, payload: Vec<u8>) -> Result<Self, Error> {
        let record = match type_id {
            FCGI_GET_VALUES => Record::GetValues(GetValues::from_record_bytes(payload)?),
            FCGI_GET_VALUES_RESULT => {
                Record::GetValuesResult(GetValuesResult::from_record_bytes(payload)?)
            }
            FCGI_BEGIN_REQUEST => Record::BeginRequest(BeginRequest::from_record_bytes(payload)?),
            FCGI_PARAMS => Record::Params(Params::from_record_bytes(payload)?),
            FCGI_STDIN => Record::Stdin(Stdin::from_record_bytes(payload)?),
            FCGI_DATA => Record::Data(Data::from_record_bytes(payload)?),
            FCGI_STDOUT => Record::Stdout(Stdout::from_record_bytes(payload)?),
            FCGI_STDERR => Record::Stderr(Stderr::from_record_bytes(payload)?),
            FCGI_ABORT_REQUEST => Record::AbortRequest(AbortRequest::from_record_bytes(payload)?),
            FCGI_END_REQUEST => Record::EndRequest(EndRequest::from_record_bytes(payload)?),
            _ => Record::UnknownType(UnknownType::from_record_bytes(payload)?),
        };

        Ok(record)
    }

    pub fn write_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        match self {
            Self::GetValues(r) => r.write_record_bytes(writer),
            Self::GetValuesResult(r) => r.write_record_bytes(writer),
            Self::BeginRequest(r) => r.write_record_bytes(writer),
            Self::Params(r) => r.write_record_bytes(writer),
            Self::Stdin(r) => r.write_record_bytes(writer),
            Self::Data(r) => r.write_record_bytes(writer),
            Self::Stdout(r) => r.write_record_bytes(writer),
            Self::Stderr(r) => r.write_record_bytes(writer),
            Self::AbortRequest(r) => r.write_record_bytes(writer),
            Self::EndRequest(r) => r.write_record_bytes(writer),
            Self::UnknownType(r) => r.write_record_bytes(writer),
        }
    }
}
