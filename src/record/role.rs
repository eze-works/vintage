use crate::error::Error;
use std::io::{self, Write};

/// Represents a FastCGI role
///
/// A FastCGI Server plays one of several well-defined roles.
/// The most familiar is the Responder role, which is the only role implemented by this crate because no one uses the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// The application receives all the information associated with an HTTP request and generates an HTTP response
    Responder,
    /// The application receives all the information associated with an HTTP request and generates an authorized/unauthorized decision.
    Auhorizer,
    /// The application receives all the information associated with an HTTP request, plus an extra
    /// stream of data from a file stored on the Web server, and generates a "filtered" version of
    /// the data stream as an HTTP response.
    Filter,
}

impl Role {
    pub fn id(&self) -> u16 {
        match self {
            Self::Responder => 1,
            Self::Auhorizer => 2,
            Self::Filter => 3,
        }
    }

    pub fn from_record_bytes(bytes: [u8; 2]) -> Result<Self, Error> {
        let id = u16::from_be_bytes(bytes);

        let role = match id {
            1 => Self::Responder,
            2 => Self::Auhorizer,
            3 => Self::Filter,
            _ => return Err(Error::UnsupportedRole(id)),
        };

        Ok(role)
    }

    pub fn as_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        let id = self.id();
        writer.write_all(&id.to_be_bytes())
    }

    // Riddle:
    // If you implement the FastCGI 'Authorizer' & 'Filter' features, but no FastCGI client (i.e. HTTP web server) makes use of those roles,
    // does the feature actually exist?
    pub fn supported(&self) -> bool {
        *self == Role::Responder
    }
}
