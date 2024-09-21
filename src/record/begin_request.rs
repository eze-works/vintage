use super::role::Role;
use crate::error::Error;
use std::io::{self, Write};

const MASK_FCGI_KEEP_CONN: u8 = 0x01;

/// A FastCGI `FCGI_BEGIN_REQUEST` record
///
/// The FastCGI client sends a FCGI_BEGIN_REQUEST record to start a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BeginRequest {
    role: Role,
    flags: u8,
}

impl BeginRequest {
    pub fn from_record_bytes(bytes: Vec<u8>) -> Result<Self, Error> {
        let [role_1, role_0, flags, ..]: [u8; 8] = bytes
            .try_into()
            .map_err(|_| Error::MalformedRecordPayload("BeginRequest"))?;

        let role = Role::from_record_bytes([role_1, role_0])?;

        if !role.supported() {
            return Err(Error::UnsupportedRole(role.id()));
        }

        Ok(BeginRequest { role, flags })
    }

    pub fn write_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        self.role.as_record_bytes(writer)?;
        writer.write_all(&[self.flags, 0, 0, 0, 0, 0])
    }

    pub fn keep_alive(&self) -> bool {
        self.flags & MASK_FCGI_KEEP_CONN == 1
    }

    #[cfg(test)]
    pub fn new(role: Role, keep_alive: bool) -> Self {
        let flags = if keep_alive { 1 } else { 0 };
        Self { role, flags }
    }
}
