//! Composing FastCGI request processing
//!
//! You can view the handling of a FastCGI request as pipeline, or sequence of steps.
//! Each step may modify the `Response`.
//! Crucially, steps have the option of either forwarding their modified `Response` to the next step, or
//! "breaking off" with their response, thus preventing subsequent steps from being run.
//!
//! The `Pipe` trait encapsulates this behavior with a `run` function returning a [`std::ops::ControlFlow`] struct.
//!
//! `Pipe`s get access to combinatorial methods that make it easy to create non-trivial request
//! pipelines.
//!
//! You are not limited to the pipes defined in this module though, as it is easy to implement
//! your own. See the [`Pipe`] trait docs for details.
mod router;

use crate::fcgi_context::FcgiContext;
pub use router::{Route, Router};
use std::ops::ControlFlow;

/// A trait for processing FastCGI requests in a composable way.
///
/// See the [module documentation](crate::pipe) for an introduction
pub trait Pipe: Sized {
    /// Run the pipe logic with the given context, and return a signal indicating if the next stage in the chain should
    /// run, or if the response should be used as is.
    fn push(&self, ctx: FcgiContext) -> ControlFlow<FcgiContext, FcgiContext>;
}

pub fn run<P: Pipe>(p: &P, ctx: FcgiContext) -> FcgiContext {
    match p.push(ctx) {
        ControlFlow::Continue(c) | ControlFlow::Break(c) => c,
    }
}
