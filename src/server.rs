mod event_loop;
mod files;
mod responder;
mod router;

use crate::context::{Request, Response};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
use std::io;
use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::mpsc::Receiver;
use std::sync::Arc;
use std::thread::JoinHandle;

type RouteParams = BTreeMap<String, String>;
type RouterCallback = Arc<dyn Fn(&mut Request, RouteParams) -> Response + Send + Sync>;

#[derive(Default, Clone)]
struct RouterSpec {
    map: BTreeMap<&'static str, matchit::Router<RouterCallback>>,
}

#[derive(Clone)]
struct FileServerSpec {
    request_prefix: String,
    fs_path: Utf8PathBuf,
}

/// Configuration of `vintage` FastCGI Server
#[derive(Clone)]
pub struct ServerSpec {
    file_server: Option<FileServerSpec>,
    router: Option<RouterSpec>,
    fallback: Option<Arc<dyn Fn(&mut Request) -> Response + Send + Sync>>,
}

impl ServerSpec {
    /// Creates a new specification for a FastCGI server
    pub fn new() -> Self {
        Self {
            file_server: None,
            router: None,
            fallback: None,
        }
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
        let request_prefix = if prefix.starts_with('/') {
            prefix.to_string()
        } else {
            format!("/{}", prefix)
        };

        let fs_path = if path.trim().is_empty() {
            Utf8PathBuf::from(".")
        } else {
            Utf8PathBuf::from(path)
        };

        // TODO: Log a warning if `fs_path` does not exist

        let spec = FileServerSpec {
            request_prefix,
            fs_path,
        };

        self.file_server = Some(spec);
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
        let callback = Arc::new(callback);
        let mut spec = self.router.unwrap_or_default();
        for path in paths {
            spec.map
                .entry(method)
                .or_default()
                .insert(path, callback.clone())
                .unwrap()
        }
        self.router = Some(spec);
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
