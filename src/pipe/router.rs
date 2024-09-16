use crate::fcgi_context::FcgiContext;
use crate::pipe::Pipe;
use crate::status;
use std::collections::BTreeMap;
use std::sync::Arc;

type RouteParams = BTreeMap<String, String>;
type RouterCallback = Arc<dyn Fn(FcgiContext, RouteParams) -> FcgiContext + Send + Sync>;
type NotFoundCallback = Box<dyn Fn(FcgiContext) -> FcgiContext + Send + Sync>;

/// A [`Pipe`] for dispatching handlers based on request method and path
pub struct Router {
    map: BTreeMap<&'static str, matchit::Router<RouterCallback>>,
    not_found_callback: NotFoundCallback,
}

impl Default for Router {
    fn default() -> Self {
        Self {
            map: BTreeMap::new(),
            not_found_callback: Box::new(|_| {
                FcgiContext::default()
                    .halt()
                    .with_raw_body(vec![])
                    .with_status(status::NOT_FOUND)
            }),
        }
    }
}

impl Router {
    /// Creates a new router
    pub fn new() -> Router {
        Self::default()
    }

    /// Registers callback tied to a set of paths & method
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
    /// ```
    /// use vintage::pipe::Router;
    ///
    /// let mut r =
    ///     Router::new()
    ///     .register("GET", ["/user/{id}/delete"], |ctx, _matched| ctx );
    /// ```
    ///
    /// _Wildcard_ matchers start with a `*` and match anything until the end of the path.
    /// They must always appear at the end of the path.
    ///
    /// In the following example, if the request was `/folder/a/b/c`, `matched["subfolders"]` would
    /// be `a/b/c`.
    ///
    /// ```
    /// use vintage::pipe::Router;
    ///
    /// let mut r =
    ///     Router::new()
    ///     .register("GET", ["/folder/{*subfolders}"], |ctx, _matched| ctx);
    /// ```
    pub fn register<C, const N: usize>(
        mut self,
        method: &'static str,
        paths: [&str; N],
        callback: C,
    ) -> Self
    where
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        let callback = Arc::new(callback);
        for path in paths {
            self.map
                .entry(method)
                .or_default()
                .insert(path, callback.clone())
                .unwrap()
        }
        self
    }

    /// Registers a callback to handle requests that result in a 404
    ///
    /// By default, the router will just send an empty response with a status code of 404
    pub fn not_found<C>(mut self, callback: C) -> Self
    where
        C: Fn(FcgiContext) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.not_found_callback = Box::new(callback);
        self
    }

    /// Registers a path for the "GET" method
    ///
    /// See [`Router::register`]
    pub fn get<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("GET", paths, callback)
    }

    /// Registers a path for the "POST" method
    ///
    /// See [`Router::register`]
    pub fn post<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("POST", paths, callback)
    }

    /// Registers a path for the "PUT" method
    ///
    /// See [`Router::register`]
    pub fn put<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("PUT", paths, callback)
    }

    /// Registers a path for the "DELETE" method
    ///
    /// See [`Router::register`]
    pub fn delete<C, const N: usize>(self, paths: [&str; N], callback: C) -> Self
    where
        C: Fn(FcgiContext, RouteParams) -> FcgiContext,
        C: 'static + Send + Sync,
    {
        self.register("DELETE", paths, callback)
    }
}

impl Pipe for Router {
    fn run(&self, ctx: FcgiContext) -> FcgiContext {
        let Some(router) = self.map.get(ctx.method()) else {
            return (self.not_found_callback)(ctx.halt());
        };

        let Ok(entry) = router.at(ctx.path()) else {
            return (self.not_found_callback)(ctx.halt());
        };

        let mut params = BTreeMap::new();

        for (key, value) in entry.params.iter() {
            params.insert(key.to_string(), value.to_string());
        }

        (entry.value)(ctx, params)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    fn make_context(method: &str, path: &str) -> FcgiContext {
        FcgiContext {
            method: method.into(),
            path: path.into(),
            ..FcgiContext::default()
        }
    }

    #[test]
    fn implementing_trailing_slash() {
        let called = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicUsize::new(0));

        let router = Router::new().get(["/path/", "/path"], {
            let counter = counter.clone();
            let called = called.clone();
            move |ctx, params| {
                assert!(params.is_empty());
                called.store(true, Ordering::SeqCst);
                counter.fetch_add(1, Ordering::SeqCst);
                ctx
            }
        });

        let request1 = make_context("GET", "/path");
        let request2 = make_context("GET", "/path/");

        let _ = router.run(request1);
        let _ = router.run(request2);

        assert_eq!(called.load(Ordering::SeqCst), true);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn wildcard_matching() {
        let called = Arc::new(AtomicBool::new(false));

        let router = Router::new().get(["/path/{*rest}"], {
            let called = called.clone();
            move |ctx, params| {
                assert_eq!(params["rest"], "a/b/c");
                called.store(true, Ordering::SeqCst);
                ctx
            }
        });

        let request = make_context("GET", "/path/a/b/c");

        let _ = router.run(request);

        assert_eq!(called.load(Ordering::SeqCst), true);
    }

    #[test]
    fn segment_matching() {
        let called = Arc::new(AtomicBool::new(false));

        let router = Router::new().get(["/path/{id}/rest"], {
            let called = called.clone();
            move |ctx, params| {
                assert_eq!(params["id"], "2");
                called.store(true, Ordering::SeqCst);
                ctx
            }
        });

        let request = make_context("GET", "/path/2/rest");

        let _ = router.run(request);

        assert_eq!(called.load(Ordering::SeqCst), true);
    }

    #[test]
    fn not_found() {
        let called = Arc::new(AtomicBool::new(false));

        let router = Router::new().not_found({
            let called = called.clone();
            move |ctx| {
                called.store(true, Ordering::SeqCst);
                ctx
            }
        });

        let request = make_context("GET", "/wut");

        let _ = router.run(request);

        assert_eq!(called.load(Ordering::SeqCst), true);
    }
}
