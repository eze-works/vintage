use crate::status;
use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::time::Instant;

/// A FastCGI request
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query_string: String,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
    pub(crate) created_at: Instant,
    pub(crate) query: OnceCell<BTreeMap<String, String>>,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            method: String::new(),
            path: String::new(),
            query_string: String::new(),
            headers: BTreeMap::new(),
            body: Vec::new(),
            created_at: Instant::now(),
            query: OnceCell::new(),
        }
    }
}

impl Request {
    /// Returns the request method
    pub fn method(&self) -> &str {
        self.method.as_str()
    }

    /// Returns the request path
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Looks up the header value associated with `key`, if any
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(key).map(String::as_str)
    }

    /// Returns a reference to the request body
    pub fn body(&self) -> &[u8] {
        self.body.as_slice()
    }

    /// Returns the request body as an owned `Vec`
    ///
    /// Once the request body has been `take`n, subsequent calls return an empty `Vec`
    pub fn take_body(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.body)
    }
}

impl Request {
    fn parse_query(qs: &str) -> BTreeMap<String, String> {
        let mut query = BTreeMap::new();
        for (k, v) in form_urlencoded::parse(qs.as_bytes()) {
            query.insert(k.to_string(), v.to_string());
        }

        query
    }

    /// Returns the value of `key` from the parsed query string
    pub fn query(&self, key: &str) -> Option<&str> {
        let map = self
            .query
            .get_or_init(|| Self::parse_query(&self.query_string));

        map.get(key).map(String::as_str)
    }
}

/// A FastCGI response
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response {
    pub(crate) status: u16,
    pub(crate) headers: BTreeMap<String, String>,
    pub(crate) body: Vec<u8>,
}

impl Default for Response {
    fn default() -> Self {
        Self {
            // The CGI RFC says this is the default if no status is provided
            status: 200,
            headers: BTreeMap::new(),
            body: Vec::new(),
        }
    }
}

impl Response {
    /// Create an empty 200 OK FastCGI response
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the response header `key` to `value`
    ///
    /// If `key` was already present in the map, the value is updated
    pub fn set_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Sets the status code of the response to `code`
    pub fn set_status(mut self, code: u16) -> Self {
        self.status = code;
        self
    }

    /// Sets the response body
    pub fn set_body(self, body: impl Into<String>) -> Self {
        self.set_raw_body(body.into().into_bytes())
    }

    /// Sets the response body in bytes
    pub fn set_raw_body(mut self, body: Vec<u8>) -> Self {
        self.body = body;
        self
    }

    fn of_content_type(content_type: &str, value: impl Into<String>) -> Self {
        Response::default()
            .set_header("Content-Type", content_type)
            .set_body(value)
    }

    /// Returns a new json response with the given value
    pub fn json(json: impl Into<String>) -> Self {
        Self::of_content_type("application/json", json)
    }

    /// Returns a new plain-text response with the given value
    pub fn text(plaintext: impl Into<String>) -> Self {
        Self::of_content_type("text/plain", plaintext)
    }

    /// Returns a new html response with the given value
    pub fn html(html: impl Into<String>) -> Self {
        Self::of_content_type("text/html", html)
    }

    /// Returns a new response that will trigger a temporary redirect
    ///
    /// The browser receiving the request will re-make the request with `path` as the new target
    /// with method and body unchanged.
    ///
    /// Search engines receiving this response will not attribute links to the original URL to the
    /// new resource, meaning no SEO value is transferred to the new URL.
    pub fn temporary_redirect(path: impl Into<String>) -> Self {
        Response::default()
            .set_header("Location", path)
            .set_status(status::TEMPORARY_REDIRECT)
    }

    /// Returns a new response that will trigger a permanent redirect
    ///
    /// The browser receiving the request will re-make the request with `path` as the new target
    /// with method and body unchanged.
    ///
    /// Search engines receiving this response will attribute links to the original URL to the
    /// redirected resource, passing the SEO ranking to the new URL.
    pub fn permanent_redirect(path: impl Into<String>) -> Self {
        Response::default()
            .set_header("Location", path)
            .set_status(status::PERMANENT_REDIRECT)
    }

    pub(crate) fn write_stdout_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        for (key, value) in self.headers.iter() {
            writeln!(writer, "{key}: {value}")?;
        }
        writeln!(writer, "Status: {}", self.status)?;
        writeln!(writer)?;
        writer.write_all(&self.body)
    }
}
