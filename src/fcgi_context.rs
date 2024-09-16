use crate::pipe::Pipe;
use crate::status;
use std::collections::BTreeMap;
use std::io::{self, Write};

/// Encapsulates all information about an individual FastCGI request and response.
///
/// A [`Pipe`](crate::pipe::Pipe) may also use this structure to [store](FcgiContext::with_data) data
/// to be used in later stages of the pipeline.
#[derive(Debug, Clone, Default)]
pub struct FcgiContext {
    pub(crate) method: String,
    pub(crate) path: String,
    pub(crate) query: BTreeMap<String, String>,
    pub(crate) incoming_headers: BTreeMap<String, String>,
    pub(crate) incoming_body: Vec<u8>,
    pub(crate) outgoing_headers: BTreeMap<String, String>,
    pub(crate) outgoing_body: Vec<u8>,
    pub(crate) data: BTreeMap<&'static str, String>,
}

impl FcgiContext {
    /// Returns a reference to data previously [stored](FcgiContext::with_data).
    pub fn get_data(&self, key: &str) -> Option<&str> {
        self.data.get(key).map(|s| s.as_str())
    }

    /// Returns a mutable reference to data previously [stored](FcgiContext::with_data).
    pub fn get_mut_data(&mut self, key: &str) -> Option<&mut str> {
        self.data.get_mut(key).map(|s| s.as_mut_str())
    }

    /// Returns the request method
    pub fn method(&self) -> &str {
        self.method.as_str()
    }

    /// Returns the request path
    pub fn path(&self) -> &str {
        self.path.as_str()
    }

    /// Returns the value of a key from the query string
    pub fn query_value(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(String::as_str)
    }

    /// Returns the value of the request header `name` if it exists
    pub fn get_header(&self, header: &str) -> Option<&str> {
        self.incoming_headers.get(header).map(String::as_str)
    }

    /// Returns the request body.
    pub fn body(&self) -> &[u8] {
        self.incoming_body.as_slice()
    }

    /// Returns a new context with the response `Content-Type` header set
    pub fn with_content_type<S: Into<String>>(self, content_type: S) -> Self {
        self.with_header("Content-Type", content_type)
    }

    /// Returns a new context with the response status set
    pub fn with_status(self, code: u16) -> Self {
        self.with_header("Status", code.to_string())
    }

    /// Returns a new context with the location response header set
    pub fn with_location<S: Into<String>>(self, location: S) -> Self {
        self.with_header("Location", location)
    }

    /// Returns a new context with the response body set
    pub fn with_body<S: Into<String>>(mut self, body: S) -> Self {
        self.outgoing_body = body.into().into_bytes();
        self
    }

    /// Returns a new context with the response body set using bytes
    pub fn with_raw_body<S: Into<Vec<u8>>>(mut self, body: S) -> Self {
        self.outgoing_body = body.into();
        self
    }

    /// Returns a new context with the response body set and the content type set to `text/html`.
    pub fn with_html_body<S: Into<String>>(self, html: S) -> Self {
        self.with_content_type("text/html").with_body(html)
    }

    /// Returns a new context with the response body set and the content type set to
    /// `application/json`
    pub fn with_json_body<S: Into<String>>(self, json: S) -> Self {
        self.with_content_type("application/json").with_body(json)
    }

    /// Returns a new context that will trigger a temporary redirect
    ///
    /// The browser receiving the request will re-make the request with `path` as the new target
    /// with method and body unchanged.
    ///
    /// Search engines receiving this response will attribute links to the original URL to the
    /// redirected resource, passing the SEO ranking to the new URL.
    pub fn with_permanent_redirect<S: Into<String>>(self, path: S) -> Self {
        self.with_status(status::PERMANENT_REDIRECT)
            .with_location(path)
    }

    /// Returns a new context that will trigger a temporary redirect
    ///
    /// The browser receiving the request will re-make the request with `path` as the new target
    /// with method and body unchanged.
    ///
    /// Search engines receiving this response will not attribute links to the original URL to the
    /// new resource, meaning no SEO value is transferred to the new URL.
    pub fn with_temporary_redirect<S: Into<String>>(self, path: S) -> Self {
        self.with_status(status::TEMPORARY_REDIRECT)
            .with_location(path)
    }

    /// Returns a new context with the given header set.
    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.outgoing_headers.insert(key.into(), value.into());
        self
    }

    /// Store data, to be used in later stages of a request pipeline
    ///
    /// This overwrites any previous value of the same type.
    pub fn with_data(mut self, key: &'static str, value: impl Into<String>) -> Self {
        self.data.insert(key, value.into());
        self
    }
}

impl FcgiContext {
    pub(crate) fn write_stdout_bytes<W: Write>(&self, writer: &mut W) -> Result<(), io::Error> {
        for (key, value) in self.outgoing_headers.iter() {
            writeln!(writer, "{key}: {value}")?;
        }
        writeln!(writer)?;
        writer.write_all(&self.outgoing_body)
    }
}

impl Pipe for FcgiContext {
    /// A context implements [`Pipe`] by returning itself.
    fn run(&self, _ctx: FcgiContext) -> Option<FcgiContext> {
        Some(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn assert_serialized(ctx: FcgiContext, expected: &str) {
        let mut buf = vec![];
        ctx.write_stdout_bytes(&mut buf).unwrap();
        assert_eq!(String::from_utf8_lossy(&buf), expected);
    }

    #[test]
    fn setting_the_status() {
        assert_serialized(FcgiContext::default().with_status(400), "Status: 400\n\n");
    }

    #[test]
    fn setting_the_location() {
        assert_serialized(
            FcgiContext::default().with_location("/path"),
            "Location: /path\n\n",
        )
    }

    #[test]
    fn setting_redirects() {
        assert_serialized(
            FcgiContext::default().with_temporary_redirect("/path"),
            "Location: /path\nStatus: 307\n\n",
        );
        assert_serialized(
            FcgiContext::default().with_permanent_redirect("/path"),
            "Location: /path\nStatus: 308\n\n",
        );
    }
    #[test]
    fn setting_the_content_type() {
        assert_serialized(
            FcgiContext::default().with_content_type("text/pre"),
            "Content-Type: text/pre\n\n",
        );
    }

    #[test]
    fn setting_body() {
        assert_serialized(FcgiContext::default().with_body("hello"), "\nhello")
    }

    #[test]
    fn setting_html_body() {
        assert_serialized(
            FcgiContext::default().with_html_body("<div></div>"),
            "Content-Type: text/html\n\n<div></div>",
        )
    }

    #[test]
    fn setting_json_body() {
        assert_serialized(
            FcgiContext::default().with_json_body("{}"),
            "Content-Type: application/json\n\n{}",
        );
    }
}
