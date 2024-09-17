//! Composing FastCGI requests
//!
//! You can view the handling of a FastCGI request as pipeline, or sequence of steps.
//! Each step may either modify and return the [request context](crate::fcgi_context::FcgiContext), or fail.
//! The `Pipe` trait encapsulates this behavior.
//!
//! You are not limited to the pipes defined in this module as it is easy to implement your own.
//! See the [`Pipe`] trait docs for details.
mod custom;
mod file_server;
mod logger;
mod router;

use crate::fcgi_context::FcgiContext;
pub use custom::custom;
pub use file_server::FileServer;
pub use logger::Logger;
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

/// Expresses an optional pipe.
///
/// Returns a new pipe that will try to run `inner` and return its result.
/// Should `inner` fail, the original context is returned
pub fn optional<P: Pipe>(inner: P) -> Optional<P> {
    Optional { pipe: inner }
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

/// See [`optional`]
pub struct Optional<P> {
    pipe: P,
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

impl<P> Pipe for Optional<P>
where
    P: Pipe,
{
    fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
        let cloned = ctx.clone();
        if let Some(result) = self.pipe.run(cloned) {
            return Some(result);
        }
        Some(ctx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    // Create two simple pipes for testing
    // One pipe only succeeds if the request method is "GET".
    // The other pipe only succeeds if the request path is "/path"
    //
    // Both pipes set a flag so it is possible to tell which of them succeeded

    struct MethodPipe;
    struct PathPipe;
    #[derive(Debug)]
    struct MethodPipeExecuted;
    #[derive(Debug)]
    struct PathPipeExecuted;

    impl Pipe for MethodPipe {
        fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
            if ctx.method() == "GET" {
                Some(ctx.with_data(MethodPipeExecuted))
            } else {
                None
            }
        }
    }

    impl Pipe for PathPipe {
        fn run(&self, ctx: FcgiContext) -> Option<FcgiContext> {
            if ctx.path() == "/path" {
                Some(ctx.with_data(PathPipeExecuted))
            } else {
                None
            }
        }
    }

    #[test]
    fn pipe_and() {
        let pipe = MethodPipe.and(PathPipe);

        // Both fail.
        let ctx = FcgiContext::new();
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // First succeeds, second fails
        let ctx = FcgiContext {
            method: "GET".into(),
            ..FcgiContext::new()
        };
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // Both succeed
        let ctx = FcgiContext {
            method: "GET".into(),
            path: "/path".into(),
            ..FcgiContext::new()
        };
        let result = pipe.run(ctx).unwrap();
        assert_matches!(result.data::<MethodPipeExecuted>(), Some(_));
        assert_matches!(result.data::<PathPipeExecuted>(), Some(_));
    }

    #[test]
    fn pipe_or() {
        let pipe = MethodPipe.or(PathPipe);

        // Both fail
        let ctx = FcgiContext::new();
        let result = pipe.run(ctx);
        assert!(result.is_none());

        // First succeeds, second does not get run
        let ctx = FcgiContext {
            method: "GET".into(),
            ..FcgiContext::new()
        };
        let result = pipe.run(ctx).unwrap();
        assert_matches!(result.data::<MethodPipeExecuted>(), Some(_));
        assert_matches!(result.data::<PathPipeExecuted>(), None);

        // First fails, second succeeds
        let ctx = FcgiContext {
            path: "/path".into(),
            ..FcgiContext::new()
        };
        let result = pipe.run(ctx).unwrap();
        assert_matches!(result.data::<MethodPipeExecuted>(), None);
        assert_matches!(result.data::<PathPipeExecuted>(), Some(_));
    }

    #[test]
    fn optional_pipe() {
        let pipe = optional(MethodPipe).and(PathPipe);

        // First fails, second still runs
        let ctx = FcgiContext {
            path: "/path".into(),
            ..FcgiContext::new()
        };

        let result = pipe.run(ctx).unwrap();

        assert_matches!(result.data::<MethodPipeExecuted>(), None);
        assert_matches!(result.data::<PathPipeExecuted>(), Some(_));


    }
}
