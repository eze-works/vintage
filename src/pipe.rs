//! Composing FastCGI request processing
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
mod router;

use crate::fcgi_context::FcgiContext;
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
/// [`FcgiContext::halt()`](crate::FcgiContext) is called, which short-circuits the pipeline.
pub trait Pipe: Sized {
    /// Run the pipe logic with the given context, and return a signal indicating if the next stage in the chain should
    /// run, or if the response should be used as is.
    fn push(&self, ctx: FcgiContext) -> FcgiContext;
}
