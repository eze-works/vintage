use super::RouterSpec;
use crate::context::{Request, Response};
use std::collections::BTreeMap;

pub fn respond(req: &mut Request, spec: RouterSpec) -> Option<Response> {
    let router = spec.map.get(req.method.as_str())?;

    let entry = router.at(&req.path).ok()?;

    let mut params = BTreeMap::new();

    for (key, value) in entry.params.iter() {
        params.insert(key.to_string(), value.to_string());
    }

    Some((entry.value)(req, params))
}
