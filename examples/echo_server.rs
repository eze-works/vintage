use vintage::{Request, Response};

fn handler1(mut request: Request) -> Response {
    dbg!(&request);
    Response::new()
        .status(200)
        .content_type("text/html")
        .body("THIS IS HANDLER 1")
}

fn handler2(mut request: Request) -> Response {
    dbg!(&request);
    Response::new()
        .status(200)
        .content_type("text/html")
        .body("THIS IS HANDLER 2")
}

fn main() {
    env_logger::init();
    let server1 = vintage::start("localhost:8000", handler1).unwrap();
    let server2 = vintage::start("localhost:8001", handler2).unwrap();
    println!("BOTH SERVERS STARTED");

    server1.stop();
    server2.stop();
}
