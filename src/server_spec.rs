use crate::context::{Request, Response};
use camino::Utf8PathBuf;
use std::collections::BTreeMap;
use std::net::SocketAddr;

/// Configuration of `vintage` FastCGI Server
pub struct ServerSpec {
    address: SocketAddr,
    file_server: Option<FileServerSpec>,
    router: Option<RouterSpec>,
}

struct FileServerSpec {
    request_prefix: &'static str,
    fs_path: Utf8PathBuf,
}

type RouteParams = BTreeMap<String, String>;
type RouterCallback = Arc<dyn Fn(Request) -> Response + Send + Sync>;

#[derive(Debug, Default)]
struct RouterSpec {
    map: BTreeMap<&'static str, matchit::Router<RouterCallback>>,
}

impl ServerSpec {
    /// Creates the specification of a new FastCGI server that will be bound to the given address
    pub fn new(address: SocketAddr) -> Self {
        Self { address }
    }

    /// Adds support for serving static files
    ///
    /// Matches requests that start with `prefix` and uses the rest of that path to lookup and
    /// serve a file from `path`
    ///
    /// If `prefix` does not begin with a forward slash (e.g. `/static`), it is implied.
    /// An empty or relative `path` implies the current working directory
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

        let spec = FileServerSpec {
            request_prefix,
            fs_path,
        };

        self.file_server = Some(spec);
        self
    }

    /// Registers a callback tied to a set of `paths` and `method`
    ///
    ///
    /// The paths support basic path segment matching.
    /// Matched path segments are passed to the callback as a second argument.
    ///
    /// # Path Matching Syntax
    ///
    /// _Segment_ matchers look like `/{id}/whatever`.
    /// They match  nything until the next `/` or the end of the path.
    /// They must match a complete segment.
    /// Suffixes/prefixes are not supported.
    ///
    /// TODO: Example
    ///
    /// _Wildcard_ matchers start with a `*` and match anything until the end of the path.
    /// They must always appear at the end of the path.
    ///
    /// In the following example, if the request was `/folder/a/b/c`, `matched["subfolders"]` would
    /// be `a/b/c`.
    ///
    /// TODO: Example
    pub fn on<C, const N: usize>(
        mut self,
        method: &'static str,
        paths: [&str; N],
        callback: C,
    ) -> Self
    where
        C: Fn(Request, RouteParams) -> Response,
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
        C: Fn(Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("GET", paths, callback)
    }

    /// Registers a path for the "POST" method
    ///
    /// See [`ServerSpec::on`]
    pub fn on_post<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("POST", paths, callback)
    }

    /// Registers a path for the "PUT" method
    ///
    /// See [`ServerSpec::on`]
    pub fn on_put<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("PUT", paths, callback)
    }

    /// Registers a path for the "DELETE" method
    ///
    /// See [`ServerSpec::on`]
    pub fn on_delete<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(Request, RouteParams) -> Response,
        C: 'static + Send + Sync,
    {
        self.on("DELETE", paths, callback)
    }
}
