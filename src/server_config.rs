use crate::context::{Request, Response};
use crate::file_server::FileServer;
use crate::router::{RouteParams, Router};
use std::sync::Arc;

/// Configuration for a `vintage` FastCGI Server
type FallbackCallback = Arc<dyn Fn(&mut Request) -> Response + Send + Sync>;

#[derive(Clone, Default)]
pub struct ServerConfig {
    pub(crate) file_server: Option<FileServer>,
    pub(crate) router: Option<Router>,
    pub(crate) fallback: Option<FallbackCallback>,
}

impl ServerConfig {
    /// Creates a new specification for a FastCGI server
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds support for serving static files
    ///
    /// Matches requests that start with `prefix` and uses the rest of the path to lookup a file on
    /// at `path`.
    ///
    /// If `prefix` does not begin with a forward slash (e.g. `/static`), it is implied.
    /// An empty `path` implies the current working directory.
    ///
    /// # Panics
    ///
    /// Panics if `path` contains invalid utf8 values
    pub fn serve_files(mut self, prefix: &'static str, path: &'static str) -> Self {
        self.file_server = Some(FileServer::new(prefix, path));
        self
    }

    /// Registers a callback tied to a `method` and a set of `paths`.
    ///
    /// If multiple paths are provided, the callback is triggered if any of them match.
    ///
    /// Paths support basic segment matching.
    /// Matched path segments are passed to the callback as a second argument.
    ///
    /// # Path Matching Syntax
    ///
    /// _Segment_ matchers look like `/{id}/whatever`.
    /// They match everything until the next `/` or the end of the path.
    /// They must match a complete segment.
    /// Suffixes/prefixes are not supported.
    ///
    /// ```
    /// use vintage::{Response, ServerConfig};
    ///
    /// let config = ServerConfig::new()
    ///     .on("GET", ["/echo/{name}"], |_req, params| {
    ///         Response::text(&params["name"])
    ///     });
    ///
    /// let handle = vintage::start(config, "localhost:0").unwrap();
    ///
    /// handle.stop();
    /// ```
    ///
    ///
    /// _Wildcard_ matchers start with a `*` and match everything until the end of the path.
    /// As such, they must always appear at the end of the path.
    ///
    /// In the following example, if the request path was `/folder/a/b/c`, `&params["subfolders"]` would
    /// be `a/b/c`.
    ///
    /// ```
    /// use vintage::{Response, ServerConfig};
    ///
    /// let config = ServerConfig::new()
    ///     .on("GET", ["/folder/{*rest}"], |_req, params| {
    ///         Response::text(&params["rest"])
    ///     });
    ///
    /// let handle = vintage::start(config, "localhost:0").unwrap();
    ///
    /// handle.stop()
    /// ```
    pub fn on<C, const N: usize>(
        mut self,
        method: &'static str,
        paths: [&str; N],
        callback: C,
    ) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        let mut router = self.router.unwrap_or_default();
        router.register(method, paths, callback);
        self.router = Some(router);
        self
    }

    /// Registers a path for the "GET" method
    ///
    /// See [`ServerConfig::on`]
    pub fn on_get<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("GET", paths, callback)
    }

    /// Registers a path for the "POST" method
    ///
    /// See [`ServerConfig::on`]
    pub fn on_post<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("POST", paths, callback)
    }

    /// Registers a path for the "PUT" method
    ///
    /// See [`ServerConfig::on`]
    pub fn on_put<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("PUT", paths, callback)
    }

    /// Registers a path for the "DELETE" method
    ///
    /// See [`ServerConfig::on`]
    pub fn on_delete<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("DELETE", paths, callback)
    }

    /// Registers a callback that will be invoked for any unhandled requests
    pub fn unhandled<C>(mut self, callback: C) -> Self
    where
        C: Fn(&mut Request) -> Response,
        C: 'static + Send + Sync,
    {
        self.fallback = Some(Arc::new(callback));
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::Connection;
    use crate::error::Error;
    use crate::record::*;
    use assert_matches::assert_matches;
    use mio::net::TcpStream;
    use std::net::SocketAddr;

    macro_rules! records {
        ($($record:expr),* $(,)?) => {{
            #[allow(unused_mut)]
            let mut records: Vec<Record> = vec![];
            $(
                records.push($record.into());
            )*
            records
        }}
    }

    fn basic_params() -> Params {
        Params::default()
            .add("REQUEST_METHOD", "GET")
            .add("PATH_INFO", "/")
            .add("QUERY_STRING", "")
    }

    // Test that when we send `to_send` records to the server at `address`, we get back the
    // `expected` records
    #[track_caller]
    fn assert_request(address: SocketAddr, to_send: Vec<Record>, mut expected: Vec<Record>) {
        let socket = TcpStream::connect(address).unwrap();
        let mut connection = Connection::try_from(socket).unwrap();

        for record in to_send.iter() {
            connection.write_record(record).unwrap();
        }

        loop {
            if expected.is_empty() {
                let result = connection.read_record();
                assert_matches!(result, Err(Error::UnexpectedSocketClose(_)));
                break;
            }

            match connection.read_record() {
                Ok(record) => {
                    assert_eq!(record, expected.remove(0));
                }
                Err(err) => panic!("{err}"),
            }
        }
    }

    #[test]
    fn get_values() {
        let server = crate::start(ServerConfig::new(), "localhost:0").unwrap();

        assert_request(
            server.address(),
            records! {
                GetValues::default(),
            },
            records! {
                GetValuesResult::default(),
            },
        );

        assert_request(
            server.address(),
            records! {
                GetValues::default().add("FCGI_MPXS_CONNS").add("VALUE_WE_DONT_KNOW"),
            },
            records! {
                GetValuesResult::default().add("FCGI_MPXS_CONNS", "0"),
            },
        );
    }

    #[test]
    fn unsupported_keepalive() {
        let server = crate::start(ServerConfig::new(), "localhost:0").unwrap();

        assert_request(
            server.address(),
            records! {
                BeginRequest::new(Role::Responder, true),
                basic_params(),
                Stdin(vec![])
            },
            records! {
                EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported)
            },
        );
    }

    #[test]
    fn successful_responder_flow() {
        // A server that echoes the body
        let config = ServerConfig::new().unhandled(|req| {
            let body = std::mem::take(&mut req.body);
            Response::default().set_raw_body(body)
        });
        let server = crate::start(config, "localhost:0").unwrap();

        assert_request(
            server.address(),
            records! {
                BeginRequest::new(Role::Responder, false),
                basic_params(),
                Stdin(b"BAR".to_vec())
            },
            records! {
                Stdout(b"Status: 200\n\nBAR".to_vec()),
                EndRequest::new(0, ProtocolStatus::RequestComplete)
            },
        );
    }
}
