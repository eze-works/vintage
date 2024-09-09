use crate::error::Error;
use std::io::{self, Write};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Responder,
    Auhorizer,
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

    pub fn to_record_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
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
