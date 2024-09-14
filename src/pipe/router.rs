use crate::fcgi_context::FcgiContext;
use crate::pipe::Pipe;
use crate::status;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::ControlFlow;

type Callback = Box<dyn Fn(FcgiContext) -> FcgiContext + Send + Sync + 'static>;

/// A FastCGI request [`Pipe`] for dispatching handlers based on request method and path
///
/// Stores a BTreeMap of matched segments under the key [`Route`].
pub struct Router {
    map: HashMap<&'static str, matchit::Router<Callback>>,
}

impl Router {
    /// Creates a new router
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Register a callback to handle requests with `method` at `path`.
    pub fn register<P, C>(&mut self, method: &'static str, path: P, callback: C)
    where
        P: Into<String>,
        C: Fn(FcgiContext) -> FcgiContext,
        C: Send + Sync + 'static,
    {
        self.map
            .entry(method)
            .or_insert(matchit::Router::new())
            .insert(path, Box::new(callback));
    }
}

impl Pipe for Router {
    fn push(&self, mut ctx: FcgiContext) -> ControlFlow<FcgiContext, FcgiContext> {
        let Some(router) = self.map.get(ctx.method()) else {
            return ControlFlow::Break(ctx.with_status(status::METHOD_NOT_ALLOWED));
        };

        // The router result borrows the path, through `ctx`.
        // If I don't clone, the borrow checker later complains when i try to use ctx mutably.
        let path = ctx.path().to_string();

        let Ok(entry) = router.at(&path) else {
            return ControlFlow::Break(ctx.with_status(status::NOT_FOUND));
        };

        let mut route = Route(BTreeMap::new());

        for (key, value) in entry.params.iter() {
            route.0.insert(key.to_string(), value.to_string());
        }

        ctx.add_data::<Route>(route);

        let response = (entry.value)(ctx);

        ControlFlow::Continue(response)
    }
}

/// Storage for matched path segments
pub struct Route(BTreeMap<String, String>);

impl Route {
    /// Returns the value of the first paramter registered under the given key
    pub fn get(&self, key: impl AsRef<str>) -> Option<&str> {
        self.0.get(key.as_ref()).map(String::as_str)
    }
}
