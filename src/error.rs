use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("The connection socket was closed unexpectedly")]
    UnexpectedSocketClose(#[source] io::Error),

    #[error("Unsupported FastCGI version: '{0}'")]
    UnsuportedVersion(u8),

    #[error("Multiplexing multiple requests unto a single connection is not supported")]
    MultiplexingUnsupported,

    #[error("Received malfored FastCGI record for type '{0}'")]
    MalformedRecordPayload(&'static str),

    #[error("Unsuported FastCGI role: '{0}'")]
    UnsupportedRole(u16),

    #[error("Unsupported FastCGI protocol status: '{0}'")]
    UnspportedProtocolStatus(u8),

    #[error("Detected invalid utf8 in a key-value pair")]
    InvalidUtf8KeyValuePair,

    #[error("Web server sent a malformed record stream")]
    MalformedRecordStream,
}
