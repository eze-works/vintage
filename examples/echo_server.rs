use vintage::{Response, ServerConfig};
fn main() {
    let config = ServerConfig::new()
        .on_get(["/say/{greeting}"], |_req, params| {
            Response::text(&params["greeting"])
        })
        .serve_files("/assets", "/var/www");
    let _handle = vintage::start(config, "localhost:0").unwrap();
}
