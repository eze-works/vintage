//! Composing FastCGI requests
//!
//! You can view the handling of a FastCGI request as pipeline, or sequence of steps.
//! Each step may modify the `Response`.
//! Crucially, steps have the option of either forwarding their modified `Response` to the next step, or
//! "breaking off" with their response, thus preventing subsequent steps from being run.
//!
//! The `Pipe` trait encapsulates this behavior through.
//!
//! `Pipe`s get access to combinatorial methods that make it easy to create non-trivial request
//! pipelines.
//!
//! You are not limited to the pipes defined in this module though, as it is easy to implement
//! your own. See the [`Pipe`] trait docs for details.
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
/// The logic that should be executed during a request should be placed in the `run()` method.
/// It takes a shared `&self` receiver because there will usually be one copy of the `Pipe` shared
/// among connection-handling threads.
///
/// By default, the next pipe configured pipe will run, unless the
/// [`FcgiContext::halt()`](crate::FcgiContext::halt) is called, which short-circuits the pipeline.
pub trait Pipe: Sized {
    /// Run the pipe logic with the given context, and return a signal indicating if the next stage in the chain should
    /// run, or if the response should be used as is.
    fn run(&self, ctx: FcgiContext) -> FcgiContext;

    /// Expresses a sequence of pipes
    ///
    /// Returns a new pipe that will provide the result of running this pipe as arguments to the `other` pipe.
    /// If this pipe halted, the `other` pipe does not get executed.
    fn and<P: Pipe>(self, other: P) -> And<Self, P> {
        And {
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

impl<P1, P2> Pipe for And<P1, P2>
where
    P1: Pipe,
    P2: Pipe,
{
    fn run(&self, ctx: FcgiContext) -> FcgiContext {
        let result = self.first.run(ctx);
        if result.halted {
            result
        } else {
            self.second.run(result)
        }
    }
}
