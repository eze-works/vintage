use vintage::{start, Response};

fn main() {
    let server = start("localhost:8000", |_request| {
        Response::html("<h1>Hello World</h1>")
    })
    .unwrap();

    server.join();
}
