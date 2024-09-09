mod begin_request;
mod end_request;
mod get_values;
mod get_values_result;
mod pairs;
mod params;
mod protocol_status;
mod role;
mod stdin;
mod stdout;
mod unknown;

use crate::error::Error;
use begin_request::BeginRequest;
use end_request::EndRequest;
use get_values::GetValues;
use get_values_result::GetValuesResult;
use params::Params;
use std::collections::BTreeMap;
use std::io::{self, Cursor, Read, Write};
use stdin::Stdin;
use stdout::Stdout;
use unknown::UnknownType;

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

pub const DISCRETE_RECORD_TYPES: [u8; 3] =
    [FCGI_BEGIN_REQUEST, FCGI_ABORT_REQUEST, FCGI_GET_VALUES];

#[derive(Debug, Clone)]
pub enum Record {
    GetValues(GetValues),
    GetValuesResult(GetValuesResult),
    BeginRequest(BeginRequest),
    Params(Params),
    Stdin(Stdin),
    UnknownType(UnknownType),
    Stdout(Stdout),
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
            Self::UnknownType(_) => FCGI_UNKNOWN_TYPE,
            Self::Stdout(_) => FCGI_STDOUT,
            Self::EndRequest(_) => FCGI_END_REQUEST,
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
            t => Record::UnknownType(UnknownType::from_record_bytes(payload)?),
        };

        Ok(record)
    }

    pub fn to_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        // We need the payload length in order to figure out the length of the padding
        let mut payload = vec![];

        match self {
            Self::GetValues(r) => r.to_record_bytes(&mut payload),
            Self::GetValuesResult(r) => r.to_record_bytes(&mut payload),
            Self::BeginRequest(r) => r.to_record_bytes(&mut payload),
            Self::Params(r) => r.to_record_bytes(&mut payload),
            Self::Stdin(r) => r.to_record_bytes(&mut payload),
            Self::UnknownType(r) => r.to_record_bytes(&mut payload),
            Self::Stdout(r) => r.to_record_bytes(&mut payload),
            Self::EndRequest(r) => r.to_record_bytes(&mut payload),
        };

        // Length of Header + Length of Payload
        let unpadded_len = 8 + payload.len();

        // Figure out the closest factor of 8 that is greater than the unpadded length
        let padded_len = unpadded_len.div_ceil(8) * 8;

        // The amount of padding is the difference between those numers
        let padding = (padded_len - unpadded_len) as u8;

        // Version + Record type + Request ID (which is always 1)
        writer.write_all(&[1, self.type_id(), 0, 1])?;
        // Payload length
        writer.write_all(&(payload.len() as u16).to_be_bytes())?;
        // Padding length + Reserved field
        writer.write_all(&[padding, 0])?;
        // Payload
        writer.write_all(&payload)?;
        // Padding
        writer.write_all(&vec![0u8; padding as usize])?;
        // Don't forget to flush.
        writer.flush()
    }
}
