use fcgiapp::{Response, Server};

fn main() {
    let server = Server::new(|request| {
        let mut response = Response::new();
        let path = request.get("PATH_INFO").unwrap();
        let body = format!("Content-Type: text/html\n\n{path}");
        response.set_body(body.into_bytes());

        response
    }, "localhost:8000").unwrap();

    server.run();
}


