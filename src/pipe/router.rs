use crate::fcgi_context::FcgiContext;
use crate::pipe::Pipe;
use crate::status;
use std::collections::BTreeMap;
use std::collections::HashMap;
use std::ops::ControlFlow;

type Callback =
    Box<dyn Fn(FcgiContext, BTreeMap<String, String>) -> FcgiContext + Send + Sync + 'static>;

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
        C: Fn(FcgiContext, BTreeMap<String, String>) -> FcgiContext,
        C: Send + Sync + 'static,
    {
        self.map
            .entry(method)
            .or_insert(matchit::Router::new())
            .insert(path, Box::new(callback));
    }
}

impl Pipe for Router {
    fn push(&self, mut ctx: FcgiContext) -> FcgiContext {
        let Some(router) = self.map.get(ctx.method()) else {
            return ctx.halt().with_status(status::METHOD_NOT_ALLOWED);
        };

        let path = ctx.path().to_string();

        let Ok(entry) = router.at(&path) else {
            return ctx.halt().with_status(status::NOT_FOUND);
        };

        let mut params = BTreeMap::new();

        for (key, value) in entry.params.iter() {
            params.insert(key.to_string(), value.to_string());
        }

        (entry.value)(ctx, params)
    }
}

