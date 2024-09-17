use super::Pipe;
use crate::fcgi_context::FcgiContext;

#[derive(Debug, Clone)]
pub struct Logger {
    label: &'static str,
}

impl Logger {
    pub fn new(label: &'static str) -> Self {
        Self { label }
    }
}

impl Pipe for Logger {
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
        let elapsed = ctx.created_at().elapsed();

        log::info!(
            status = ctx.outgoing_status,
            method = ctx.method(),
            path = ctx.path(),
            elapsed = elapsed.as_micros();
            "{}",
            self.label
        );

        Some(ctx)
    }
}
