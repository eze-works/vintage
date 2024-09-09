use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Occurs when a [`Connection`] could not be created from an underlying transport
    #[error("Could not duplicate socket")]
    Socket(#[source] io::Error),

    /// Occurs if reading from a [`Connection`] fails in the middle of a request
    #[error("The connection socket was closed unexpectedly")]
    UnexpectedSocketClose(#[source] io::Error),

    /// Occurs if FastCgi record uses any version other than "1"
    #[error("Unsupported FastCGI version: '{0}'")]
    UnsuportedVersion(u8),

    #[error("Multiplexing multiple requests unto a single connection is not supported")]
    MultiplexingUnsupported,

    /// Occurs when a record type is recognized but its payload was malformed
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
