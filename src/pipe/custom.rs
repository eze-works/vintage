use super::Pipe;
use crate::fcgi_context::FcgiContext;

/// Returns a new [`Pipe`] that executes the given callback when it runs.
///
/// This is shortcut for definining a simple pipe without creating an new type and implementing the
/// trait.
pub fn custom<F>(callback: F) -> Custom<F>
where
    F: Fn(FcgiContext) -> Option<FcgiContext>,
{
    Custom { callback }
}

pub struct Custom<F> {
    callback: F,
}

impl<F> Pipe for Custom<F>
where
    F: Fn(FcgiContext) -> Option<FcgiContext>,
{
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
        (self.callback)(ctx)
    }
}
