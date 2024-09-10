use std::io;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Occurs if reading from a [`Connection`] fails in the middle of a request
    #[error("The connection socket was closed unexpectedly")]
    UnexpectedSocketClose(#[source] io::Error),

    /// Occurs if FastCgi record uses any version other than "1"
    #[error("Unsupported FastCGI version: '{0}'")]
    UnsuportedVersion(u8),

    /// Occurs when we observe a request id greater than 1 on a connection. This implies the client
    /// tried to multiplex the connection, which is not supported by this crate.
    #[error("Multiplexing multiple requests unto a single connection is not supported")]
    MultiplexingUnsupported,

    /// Occurs when a record type is recognized but its payload was malformed
    #[error("Received malfored FastCGI record for type '{0}'")]
    MalformedRecordPayload(&'static str),

    /// Occurs when deserializing a [`BeginRequest`](crate::record::BeginRequest) record with an unrecognized role
    #[error("Unsuported FastCGI role: '{0}'")]
    UnsupportedRole(u16),

    /// Occurs when deserializing a [`EndRequest`](crate::record::EndRequest) record with an
    /// unrecognized protocol status.
    #[error("Unsupported FastCGI protocol status: '{0}'")]
    UnspportedProtocolStatus(u8),

    /// Occurs when deserializing a key value pair that contains invalid utf8
    #[error("Detected invalid utf8 in a key-value pair")]
    InvalidUtf8KeyValuePair,

    /// Occurs when a stream ends without a final empty packet
    #[error("Web server sent a malformed record stream")]
    MalformedRecordStream,
}
