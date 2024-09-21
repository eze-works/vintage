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
        let router = self.map.get(req.method())?;

        let entry = router.at(req.path()).ok()?;

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

    fn make_request(method: &str, path: &str) -> Request {
        Request {
            method: method.into(),
            path: path.into(),
            ..Request::default()
        }
    }

    #[test]
    fn non_matching_method() {
        let mut router = Router::default();
        router.register("GET", ["/path"], move |_req, _params| Response::default());

        let mut request = make_request("POST", "/path");
        let response = router.respond(&mut request);

        assert_eq!(response, None);
    }

    #[test]
    fn non_matching_path() {
        let mut router = Router::default();
        router.register("GET", ["/path"], move |_req, _params| Response::default());

        let mut request = make_request("GET", "/rong");
        let response = router.respond(&mut request);

        assert_eq!(response, None);
    }

    #[test]
    fn implementing_trailing_slash() {
        let mut router = Router::default();
        router.register("GET", ["/path/", "/path"], move |_req, _params| {
            Response::default().set_status(100)
        });

        let mut request1 = make_request("GET", "/path");
        let mut request2 = make_request("GET", "/path/");

        let response1 = router.respond(&mut request1).unwrap();
        let response2 = router.respond(&mut request2).unwrap();

        assert_eq!(response1, Response::default().set_status(100));
        assert_eq!(response2, Response::default().set_status(100));
    }

    #[test]
    fn wildcard_matching() {
        let mut router = Router::default();
        router.register("GET", ["/path/{*rest}"], move |_req, params| {
            Response::default().set_body(&params["rest"])
        });

        let mut request = make_request("GET", "/path/a/b/c");
        let response = router.respond(&mut request).unwrap();

        assert_eq!(response, Response::default().set_body("a/b/c"));
    }

    #[test]
    fn segment_matching() {
        let mut router = Router::default();
        router.register("GET", ["/path/{id}/rest"], {
            move |_req, params| Response::default().set_body(&params["id"])
        });

        let mut request = make_request("GET", "/path/2/rest");

        let response = router.respond(&mut request).unwrap();

        assert_eq!(response, Response::default().set_body(String::from("2")));
    }
}
