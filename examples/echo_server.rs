use vintage::{Request, Response};

fn handler1(mut request: Request) -> Response {
    dbg!(&request);
    Response::html("<h1>Hello World</h1>")
}

fn handler2(mut request: Request) -> Response {
    dbg!(&request);
    Response::text("Hello world")
}

fn handler3(mut request: Request) -> Response {
    Response::json("[1]")
}

fn main() {
    env_logger::init();
    let server1 = vintage::start("localhost:8000", handler1).unwrap();
    let server2 = vintage::start("localhost:8001", handler2).unwrap();
    let server3 = vintage::start("localhost:8002", handler3).unwrap();

    println!("ALL SERVERS STARTED");

    loop {}

    // server1.stop();
    // server2.stop();
    // server3.stop();
}
