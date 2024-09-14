use vintage::pipe::{Pipe, Router};
use vintage::{start, status};

fn main() {
    let mut router = Router::new();
    router.register("GET", "/echo/{msg}", |ctx, params| {
        let msg = &params["msg"];
        ctx.with_html_body(msg).with_status(200)
    });
    router.register("GET", "/greet", |ctx, _| {
        ctx.with_html_body("<h1>Hello World</h1>").with_status(200)
    });

    let server = start("localhost:8000", move |ctx| router.push(ctx)).unwrap();

    server.join();
}
