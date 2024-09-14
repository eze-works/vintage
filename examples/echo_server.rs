use vintage::pipe::{run, Route, Router};
use vintage::{start, status};

fn main() {
    let mut router = Router::new();
    router.register("GET", "/echo/{msg}", |ctx| {
        let msg = ctx
            .get_data::<Route>()
            .unwrap()
            .get("msg")
            .unwrap()
            .to_string();
        ctx.with_html_body(msg).with_status(200)
    });
    router.register("GET", "/greet", |ctx| {
        ctx.with_html_body("<h1>Hello World</h1>").with_status(200)
    });

    let server = start("localhost:8000", move |ctx| run(&router, ctx)).unwrap();

    server.join();
}
