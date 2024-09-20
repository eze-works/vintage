mod event_loop;
mod responder;

use crate::context::{Request, Response};
use crate::file_server::FileServer;
use crate::router::{RouteParams, Router};
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

/// Configuration for a `vintage` FastCGI Server
type FallbackCallback = Arc<dyn Fn(&mut Request) -> Response + Send + Sync>;

#[derive(Clone, Default)]
pub struct ServerSpec {
    file_server: Option<FileServer>,
    router: Option<Router>,
    fallback: Option<FallbackCallback>,
}

impl ServerSpec {
    /// Creates a new specification for a FastCGI server
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts the FastCGI server at `address` and returns a handle to it.
    ///
    /// Binding to port `0` will request that the OS assign an available port.
    ///
    /// If `address` yields multiple addresses, only the first one is considered.
    ///
    /// This function does not block because the FastCGI server is created on a separate thread.
    pub fn start(self, address: impl ToSocketAddrs) -> Result<ServerHandle, io::Error> {
        let mut iter = address.to_socket_addrs()?;
        let first_address = iter
            .next()
            .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
        event_loop::create_handle(self, first_address)
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
    /// use vintage::{Response, ServerSpec};
    ///
    /// let handle = ServerSpec::new()
    ///     .on("GET", ["/echo/{name}"], |_req, params| {
    ///         Response::text(&params["name"])
    ///     })
    ///     .start("localhost:0")
    ///     .unwrap();
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
    /// use vintage::{Response, ServerSpec};
    ///
    /// let handle = ServerSpec::new()
    ///     .on("GET", ["/folder/{*rest}"], |_req, params| {
    ///         Response::text(&params["rest"])
    ///     })
    ///     .start("localhost:0")
    ///     .unwrap();
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
    /// See [`ServerSpec::on`]
    pub fn on_get<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("GET", paths, callback)
    }

    /// Registers a path for the "POST" method
    ///
    /// See [`ServerSpec::on`]
    pub fn on_post<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("POST", paths, callback)
    }

    /// Registers a path for the "PUT" method
    ///
    /// See [`ServerSpec::on`]
    pub fn on_put<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(&mut Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("PUT", paths, callback)
    }

    /// Registers a path for the "DELETE" method
    ///
    /// See [`ServerSpec::on`]
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

/// The reason the server exited
#[derive(Debug, Default)]
pub enum ServerExitReason {
    /// It was gracefully shutdown
    #[default]
    Normal,
    /// Polling the server socket for new connections failed somehow.
    Err(io::Error),
    /// The server panicked. The payload will contain the panic message.
    Panic(String),
}

/// Handle to a running FastCGI server
pub struct ServerHandle {
    address: SocketAddr,
    server_loop: JoinHandle<ServerExitReason>,
    server_waker: mio::Waker,
    observe_shutdown: Receiver<()>,
}

impl ServerHandle {
    /// Blocks until the server terminates and returns the reason.
    ///
    /// This function does not attempt to stop the server.
    /// It waits (potentially indefinitely) until the server exits.
    /// If you want to stop sthe server, use [`stop()`](crate::ServerHandle::stop).
    pub fn join(self) -> ServerExitReason {
        match self.server_loop.join() {
            Ok(r) => r,
            Err(any) => match any.as_ref().downcast_ref::<String>() {
                Some(s) => ServerExitReason::Panic(s.clone()),
                None => match any.as_ref().downcast_ref::<&str>() {
                    Some(s) => ServerExitReason::Panic(s.to_string()),
                    None => ServerExitReason::Panic(String::new()),
                },
            },
        }
    }

    /// Stops the FastCGI server
    ///
    /// The server waits for all in-flight requests to complete before it is shutdown
    pub fn stop(self) {
        // Wake up the server thread.
        // It will be able to tell that it was woken up by the waker instead of by a new readable Tcp connection.
        // If this call fails, just return.
        // We don't want to attempt to block on the `recv()` call in the next line if its possible
        // we didn't wake the server.
        // This means our graceful shutdown is "best effort".
        // Nothing we can do if some OS-level error happened.
        let Ok(()) = self.server_waker.wake() else {
            return;
        };

        // Normally, after the server thread is woken up by the waker, it will eventually
        // rendezvous here.
        // Except if it exited due to an error or panicked, in which case this call would return
        // with an error. But we ignore it because we only care that the server loop is stopped.
        let _ = self.observe_shutdown.recv();
    }

    /// Returns the address at which the server is currently listening
    pub fn address(&self) -> SocketAddr {
        self.address
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
        let server = ServerSpec::new().start("localhost:0").unwrap();

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
        let server = ServerSpec::new().start("localhost:0").unwrap();

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
        let server = ServerSpec::new()
            .unhandled(|req| {
                let body = std::mem::take(&mut req.body);
                Response::default().set_raw_body(body)
            })
            .start("localhost:0")
            .unwrap();

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
