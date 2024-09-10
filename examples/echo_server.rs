use fcgiapp::{Response, Server};

fn main() {
    let server = Server::new(
        |request| {
            let path = request.path_info().unwrap();
            let mut response = Response::new()
                .status(200)
                .content_type("text/html")
                .body(path);
            response
        },
        "localhost:8000",
    )
    .unwrap();

    server.run();
}
