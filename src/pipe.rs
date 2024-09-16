//! Composing FastCGI requests
//!
//! You can view the handling of a FastCGI request as pipeline, or sequence of steps.
//! Each step may either modify and return the [request context](crate::fcgi_context::FcgiContext), or fail.
//! The `Pipe` trait encapsulates this behavior.
//!
//! You are not limited to the pipes defined in this module as it is easy to implement your own.
//! See the [`Pipe`] trait docs for details.
mod file_server;
mod router;

use crate::fcgi_context::FcgiContext;
pub use file_server::FileServer;
pub use router::Router;

/// A trait for processing FastCGI requests in a composable way.
///
/// See the [module documentation](crate::pipe) for an introduction
///
/// # Implementing the trait
///
/// The logic to be executed during a request should be placed in the `run()` method.
/// Returning `None` from this function means the pipe failed.
/// The function takes a shared `&self` receiver because there will usually be one copy of the `Pipe` shared
/// among connection-handling threads.
pub trait Pipe: Sized {
    /// Runs the pipe logic.
    /// Pipes indicate failure by returning `None`.
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext>;

    /// Expresses a sequence of pipes
    ///
    /// Returns a new pipe that will run this pipe and the `other` pipe in sequence, providing the result of running this pipe as arguments to the `other` pipe.
    /// If this pipe fails, the `other` pipe does not run.
    fn and<P: Pipe>(self, other: P) -> And<Self, P> {
        And {
            first: self,
            second: other,
        }
    }

    /// Expresses an alternate pipe
    ///
    /// Returns a new pipe that will return the result of running this pipe.
    /// If this pipe fails, the other pipe is tried.
    fn or<P: Pipe>(self, other: P) -> Or<Self, P> {
        Or {
            first: self,
            second: other,
        }
    }
}

/// See [`Pipe::and`]
pub struct And<P1, P2> {
    first: P1,
    second: P2,
}

/// See [`Pipe::or`]
pub struct Or<P1, P2> {
    first: P1,
    second: P2,
}

impl<P1, P2> Pipe for And<P1, P2>
where
    P1: Pipe,
    P2: Pipe,
{
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
        self.second.run(self.first.run(ctx)?)
    }
}

impl<P1, P2> Pipe for Or<P1, P2>
where
    P1: Pipe,
    P2: Pipe,
{
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
        let cloned = ctx.clone();
        if let Some(result) = self.first.run(cloned) {
            return Some(result);
        }
        self.second.run(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Create two simple pipes for testing
    // One pipe only succeeds if the request method is "GET".
    // The other pipe only succeeds if the request path is "/path"
    //
    // Both pipes set a flag so it is possible to tell which of them succeeded

    struct MethodPipe;
    struct PathPipe;

    impl Pipe for MethodPipe {
        fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
            if ctx.method() == "GET" {
                Some(ctx.with_data("Method", "true"))
            } else {
                None
            }
        }
    }

    impl Pipe for PathPipe {
        fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
            if ctx.path() == "/path" {
                Some(ctx.with_data("Path", "true"))
            } else {
                None
            }
        }
    }

    #[test]
    fn pipe_and() {
        let pipe = MethodPipe.and(PathPipe);

        // Both fail.
        let ctx = FcgiContext::default();
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // First succeeds, second fails
        let ctx = FcgiContext {
            method: "GET".into(),
            ..FcgiContext::default()
        };
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // Both succeed
        let ctx = FcgiContext {
            method: "GET".into(),
            path: "/path".into(),
            ..FcgiContext::default()
        };
        let result = pipe.run(ctx).unwrap();
        assert_eq!(result.get_data("Method"), Some("true"));
        assert_eq!(result.get_data("Path"), Some("true"));
    }

    #[test]
    fn pipe_or() {
        let pipe = MethodPipe.or(PathPipe);

        // Both fail
        let ctx = FcgiContext::default();
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // First succeeds, second does not get run
        let ctx = FcgiContext {
            method: "GET".into(),
            ..FcgiContext::default()
        };
        let result = pipe.run(ctx).unwrap();
        assert_eq!(result.get_data("Method"), Some("true"));
        assert_eq!(result.get_data("Path"), None);

        // First fails, second succeeds
        let ctx = FcgiContext {
            path: "/path".into(),
            ..FcgiContext::default()
        };
        let result = pipe.run(ctx).unwrap();
        assert_eq!(result.get_data("Method"), None);
        assert_eq!(result.get_data("Path"), Some("true"));
    }
}
