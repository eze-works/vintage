use vintage::pipe::{Pipe, Router};
use vintage::{start, FcgiContext};

fn main() {
    let fallback = FcgiContext::default()
        .with_html_body("<h1>Not Found</h1>")
        .with_status(404);
    let router = Router::new()
        .get(["/echo/{msg}"], |ctx, params| {
            let msg = &params["msg"];
            ctx.with_html_body(msg).with_status(200)
        })
        .get(["/greet"], |ctx, _| {
            ctx.with_html_body("<h1>Hello World</h1>").with_status(200)
        });

    let pipeline = router.or(fallback);
    let server = start("localhost:8000", move |ctx| pipeline.run(ctx)).unwrap();

    server.join();
}
