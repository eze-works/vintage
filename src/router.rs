use crate::context::{Request, Response};
use std::collections::BTreeMap;
use std::sync::Arc;

pub type RouteParams = BTreeMap<String, String>;
pub type RouterCallback = Arc<dyn Fn(&mut Request, RouteParams) -> Response + Send + Sync>;

#[derive(Default, Clone)]
pub struct Router {
    map: BTreeMap<&'static str, matchit::Router<RouterCallback>>,
}

impl Router {
    pub fn new() -> Self {
        Router::default()
    }

    pub fn register<C, const N: usize>(
        &mut self,
        method: &'static str,
        paths: [&str; N],
        callback: C,
    ) where
        C: Fn(&mut Request, RouteParams) -> Response,
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
    }

    pub fn respond(&self, req: &mut Request) -> Option<Response> {
        let router = self.map.get(req.method.as_str())?;

        let entry = router.at(&req.path).ok()?;

        let mut params = BTreeMap::new();

        for (key, value) in entry.params.iter() {
            params.insert(key.to_string(), value.to_string());
        }

        Some((entry.value)(req, params))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    fn make_request(method: &str, path: &str) -> Request {
        let mut req = Request::default();
        req.method = method.into();
        req.path = path.into();
        req
    }

    #[test]
    fn implementing_trailing_slash() {
        let called = Arc::new(AtomicBool::new(false));
        let counter = Arc::new(AtomicUsize::new(0));

        let mut router = Router::new();
        router.register("GET", ["/path/", "/path"], {
            let counter = counter.clone();
            let called = called.clone();
            move |_req, params| {
                assert!(params.is_empty());
                called.store(true, Ordering::SeqCst);
                counter.fetch_add(1, Ordering::SeqCst);
                Response::default()
            }
        });

        let mut request1 = make_request("GET", "/path");
        let mut request2 = make_request("GET", "/path/");

        let _ = router.respond(&mut request1);
        let _ = router.respond(&mut request2);

        assert_eq!(called.load(Ordering::SeqCst), true);
        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn wildcard_matching() {
        let called = Arc::new(AtomicBool::new(false));

        let mut router = Router::new();
        router.register("GET", ["/path/{*rest}"], {
            let called = called.clone();
            move |_req, params| {
                assert_eq!(params["rest"], "a/b/c");
                called.store(true, Ordering::SeqCst);
                Response::default()
            }
        });

        let mut request = make_request("GET", "/path/a/b/c");

        let _ = router.respond(&mut request);

        assert_eq!(called.load(Ordering::SeqCst), true);
    }

    #[test]
    fn segment_matching() {
        let called = Arc::new(AtomicBool::new(false));

        let mut router = Router::new();
        router.register("GET", ["/path/{id}/rest"], {
            let called = called.clone();
            move |_req, params| {
                assert_eq!(params["id"], "2");
                called.store(true, Ordering::SeqCst);
                Response::default()
            }
        });

        let mut request = make_request("GET", "/path/2/rest");

        let _ = router.respond(&mut request);

        assert_eq!(called.load(Ordering::SeqCst), true);
    }
}
