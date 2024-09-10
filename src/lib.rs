// Implementation notes
//
// - The API should involved a single function call that returns an instance of the server that can
// be stopped

mod connection;
mod error;
pub mod record;
mod server;

pub use server::{NoReturn, Request, Response, Server};
