use vintage::{Response, ServerSpec};
fn main() {
    let _handle = ServerSpec::new()
        .on_get(["/say/{greeting}"], |_req, params| {
            Response::text(&params["greeting"])
        })
        .serve_files("/assets", "/var/www")
        .start("localhost:0")
        .unwrap();
}
