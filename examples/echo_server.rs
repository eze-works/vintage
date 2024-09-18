use vintage::pipe::{self, Pipe};
use vintage::start;

fn main() {
    env_logger::init();
    let fallback =
        pipe::custom(move |ctx| Some(ctx.with_html_body("<h1>Not Found</h1>").with_status(404)));

    let router = pipe::Router::new()
        .get(["/echo/{msg}"], |ctx, params| {
            let msg = &params["msg"];
            ctx.with_html_body(msg).with_status(200)
        })
        .get(["/greet"], |ctx, _| {
            ctx.with_html_body("<h1>Hello World</h1>").with_status(200)
        });

    let logger = pipe::Logger::new("got request");
    let pipeline = pipe::optional(router.or(fallback)).and(logger);
    let server = start("localhost:8000", move |ctx| pipeline.apply(ctx)).unwrap();

    server.join();
}
