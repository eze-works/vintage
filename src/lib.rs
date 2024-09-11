// Implementation notes
//
// - The API should involved a single function call that returns an instance of the server that can
// be stopped

mod connection;
mod error;
mod record;
mod request;
mod response;
mod server;

pub use request::Request;
pub use response::Response;
pub use server::{start, ServerHandle};
