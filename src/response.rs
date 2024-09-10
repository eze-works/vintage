use crate::record::Stdout;
use std::collections::BTreeMap;
use std::io::Write;

/// A FastCGI response
#[derive(Debug, Clone)]
pub struct Response {
    headers: BTreeMap<String, String>,
    body: Vec<u8>,
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
    pub fn content_type(mut self, content_type: &str) -> Self {
        self.headers
            .insert("Content-Type".into(), content_type.into());
        self
    }

    /// Sets the `Location` header
    pub fn location(mut self, location: &str) -> Self {
        self.headers.insert("Location".into(), location.into());
        self
    }

    /// Sets the status of the response
    pub fn status(mut self, code: u16) -> Self {
        self.headers.insert("Status".into(), code.to_string());
        self
    }

    /// Sets the content of the body
    pub fn body(mut self, body: &str) -> Self {
        self.body = body.to_string().into_bytes();
        self
    }

    /// Sets the content of the body using raw bytes
    pub fn body_raw(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }
}

impl From<Response> for Stdout {
    fn from(mut value: Response) -> Self {
        let mut bytes = vec![];

        for (key, value) in value.headers {
            write!(&mut bytes, "{key}: {value}\n").unwrap();
        }
        write!(&mut bytes, "\n").unwrap();
        bytes.append(&mut value.body);
        Stdout::new(bytes)
    }
}
