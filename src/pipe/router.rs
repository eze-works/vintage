use crate::fcgi_context::FcgiContext;
use crate::pipe::Pipe;
use crate::status;
use std::collections::BTreeMap;

type RouteParams = BTreeMap<String, String>;
type RouterCallback = Box<dyn Fn(FcgiContext, RouteParams) -> FcgiContext + Send + Sync>;

/// A [`Pipe`] for dispatching handlers based on request method and path
#[derive(Default)]
pub struct Router {
    map: BTreeMap<&'static str, matchit::Router<RouterCallback>>,
}

impl Router {
    /// Creates a new router
    pub fn new() -> Router {
        Self {
            map: BTreeMap::new(),
        }
    }

    /// Register a callback to handle requests with `method` at `path`.
    ///
    /// The path argument to this function supports basic matching of path segments.
    /// Matched path segments are passed to the callback as a second argument.
    ///
    /// # Path Matching Syntax
    ///
    /// _Named_ matchers like `/{id}/whatever` match anything until the next `/` or the end of the
    /// path.
    /// They must match a complete segment. Suffixes/prefixes are not supported.
    ///
    /// ```
    /// use vintage::pipe::Router;
    ///
    /// let mut r =
    ///     Router::new()
    ///     .register("GET", "/user/{id}/delete", |ctx, _matched| ctx );
    /// ```
    ///
    /// _Catch-all_ matchers start with a `*` and match anything until the end of the path.
    /// They must always appear at the end of the path.
    ///
    /// ```
    /// use vintage::pipe::Router;
    ///
    /// let mut r =
    ///     Router::new()
    ///     .register("GET", "/deprecated/{*}", |ctx, _matched| ctx)
    ///     .register("GET", "/folder/{*subfolders}", |ctx, _matched| ctx);
    /// ```
    pub fn register<C, P>(mut self, method: &'static str, path: P, callback: C) -> Self
    where
        P: Into<String>,
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        let _ = self
            .map
            .entry(method)
            .or_default()
            .insert(path, Box::new(callback));
        self
    }

    /// Registers a path for the "GET" method
    ///
    /// See [`Router::register`]
    pub fn get<C, P>(self, path: P, callback: C) -> Self
    where
        P: Into<String>,
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("GET", path, callback)
    }

    /// Registers a path for the "POST" method
    ///
    /// See [`Router::register`]
    pub fn post<C, P>(self, path: P, callback: C) -> Self
    where
        P: Into<String>,
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("POST", path, callback)
    }

    /// Registers a path for the "PUT" method
    ///
    /// See [`Router::register`]
    pub fn put<C, P>(self, path: P, callback: C) -> Self
    where
        P: Into<String>,
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("PUT", path, callback)
    }

    /// Registers a path for the "DELETE" method
    ///
    /// See [`Router::register`]
    pub fn delete<C, P>(self, path: P, callback: C) -> Self
    where
        P: Into<String>,
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("DELETE", path, callback)
    }
}

impl Pipe for Router {
    fn run(&self, ctx: FcgiContext) -> FcgiContext {
        let Some(router) = self.map.get(ctx.method()) else {
            return ctx.halt().with_status(status::METHOD_NOT_ALLOWED);
        };

        let Ok(entry) = router.at(ctx.path()) else {
            return ctx.halt().with_status(status::NOT_FOUND);
        };

        let mut params = BTreeMap::new();

        for (key, value) in entry.params.iter() {
            params.insert(key.to_string(), value.to_string());
        }

        (entry.value)(ctx, params)
    }
}
