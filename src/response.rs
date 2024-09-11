use crate::record::Stdout;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::io::{self, Write};

/// A response from a FastCGI server.
#[derive(Debug)]
pub struct Response {
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
    error: Option<Box<dyn std::error::Error>>,
}

impl Default for Response {
    fn default() -> Self {
        let default_headers = BTreeMap::from_iter([
            ("Status".into(), "200".into()),
            ("Content-Type".into(), "text/plain".into()),
        ]);
        Self {
            headers: default_headers,
            body: vec![],
            error: None,
        }
    }
}

impl Response {
    /// Create a new FastCGI response.
    ///
    /// Defaults to an empty 200 OK
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the `Content-Type` header
    pub fn set_content_type<S: Display>(mut self, content_type: S) -> Self {
        self.headers
            .insert("Content-Type".into(), content_type.to_string());
        self
    }

    /// Sets the `Location` header
    pub fn set_location<S: Display>(mut self, location: S) -> Self {
        self.headers.insert("Location".into(), location.to_string());
        self
    }

    /// Sets the status of the response
    pub fn set_status(mut self, code: u16) -> Self {
        self.headers.insert("Status".into(), code.to_string());
        self
    }

    /// Sets the content of the body
    pub fn set_body<S: Display>(mut self, body: S) -> Self {
        self.body = body.to_string().into_bytes();
        self
    }

    /// Sets the content of the body using raw bytes
    pub fn set_body_raw(mut self, body: &[u8]) -> Self {
        self.body = body.to_vec();
        self
    }

    /// Sets an error associated with the response
    ///
    /// Note: The reported error is sent in a separate FastCGI "stderr" stream which usually just
    /// ends up in the logs of the FastCGI client.
    /// Meaning, this does not get reported to the browser at all.
    pub fn set_error(mut self, err: Box<dyn std::error::Error>) -> Self {
        self.error = Some(err);
        self
    }

    /// Returns the error associated with the response, if any
    pub fn get_error(&self) -> Option<&dyn std::error::Error> {
        self.error.as_deref()
    }

    pub(crate) fn write_stdout_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        for (key, value) in self.headers.iter() {
            writeln!(writer, "{key}: {value}")?;
        }
        writeln!(writer)?;
        writer.write_all(&self.body)
    }

    pub(crate) fn write_stderr_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        let Some(err) = &self.error else {
            return Ok(());
        };

        writeln!(writer, "{}", err.to_string())?;

        for child_err in std::iter::successors(Some(err.as_ref()), |err| err.source()) {
            writeln!(writer, "\t{}", child_err.to_string())?;
        }
        Ok(())
    }
}

// Shortcuts for creating responses
impl Response {
    pub fn text<S: std::fmt::Display>(str: S) -> Response {
        Response::default()
            .set_body(str.to_string())
            .set_content_type("text/plain")
    }

    /// Creates an HTML response
    pub fn html<S: Display>(str: S) -> Response {
        Response::default()
            .set_body(str.to_string())
            .set_content_type("text/html")
    }

    /// Creates a JSON response
    pub fn json<S: Display>(str: S) -> Response {
        Response::default()
            .set_body(str.to_string())
            .set_content_type("application/json")
    }
}
