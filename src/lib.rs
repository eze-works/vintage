//! A library for writing FastCGI application servers.
//! It mostly implements the [FastCGI](https://www.mit.edu/~yandros/doc/specs/fcgi-spec.html#S4)
//! spec, but [deviates](crate#deviations-from-the-spec) where it makes sense to do so.
//!
//! Using this crate is straightforward:
//!
//! ```
//! use vintage::{Response, ServerConfig};
//!
//! let config = ServerConfig::new()
//!     .on_get(["/about"], |_req, _params| {
//!         Response::html("<h1>Hello World</h1>")
//!     });
//!
//! let handle = vintage::start(config, "localhost:0").unwrap();
//!
//! // This would block the current thread until the server thread exits
//! // handle.join()
//!
//! // Gracefull shutdown
//! handle.stop();
//! ```
//!
//! # Terminology
//!
//! - CGI: A specification HTTP web servers can follow to execute a program in response to HTTP requests.
//!   For example, you would configure your web server (e.g. Apache) to execute a certain bash script when a request came in.
//!   The bash script would get access to request metadata via environment variables.
//! - FastCGI: A successor to CGI (Common Gateway Interface). Unlike CGI, programs are not executed every time a request comes in.
//!   Instead, a FastCGI application is started and listens on a socket, and the HTTP web server communicates HTTP request metadata via that socket.
//!   The FastCGI spec is a definition of the binary protocol used to communicate on that socket.
//! - FastCGI client: The program that initiates a FastCGI connection.
//!   In most cases, this is the HTTP web server; it receives an HTTP request from a browser, and forwards that request to the FastCGI server.
//! - FastCGI server/application server: A program that listens on a socket, and responds to requests from a FastCGI client.
//!
//! # Deviations from the spec
//!
//! The FastCGI spec was created by a company called [Open Market](https://en.wikipedia.org/wiki/Open_Market).
//! One of their products was a web server, and it was the first commercial web server receive FastCGI support.
//! As a consequence, their FastCGI specification includes details only relevant to their
//! web server implementation.
//! Since their web server is no more, we disregard these parts of the spec.
//! Additionally, the passage of time has made some other parts of the specification obsolete.
//!
//! Notably:
//! - I ignore the part about what file descriptors are open when the FastCGI server begins (Section 2.2)
//! - I ignore the special processing of the magic `FCGI_WEB_SERVER_ADDRS` environment variable (Section 3.2)
//! - `FCGI_UNKNOWN_TYPE` is sent for any unknown record type, instead of just unknown management
//!   record types (Section 4.2).
//! - Only the Responder role is implemented. Two reasons:
//!   - Authorizer & Filter roles are not implemented by any current FastCGI-capable servers (or clients).
//!     - I checked the source code of Nginx, Caddy and Php-fpm (arguabley the most popular fastcgi client).
//!   - Authorizer & Filter are not relevant anymore.
//!     - Authorization is usually part of the application.
//!     - The Filter is too niche to be useful. It assumes your request path has an extension.
//!       The spec is actually light on details regarding its use.
//!       OpenMarket's archived
//!       [manual](https://fastcgi-archives.github.io/fcgi2/doc/fastcgi-prog-guide/ch1intro.htm)
//!       has more info.
//! - Writing a "stderr" record is not supported. As far as I can tell, it's pretty useless.
//!   At best, what you send in that record gets printed in the logs of the FastCGI _client_.
//!   At worst, it gets ignored.

mod connection;
mod context;
mod error;
mod event_loop;
mod fastcgi_responder;
mod file_server;
mod record;
mod router;
mod server_config;
mod server_handle;
pub mod status;

pub use context::{Request, Response};
pub use server_config::ServerConfig;
pub use server_handle::{ServerExitReason, ServerHandle};

use std::io;
use std::net::ToSocketAddrs;

/// Starts a FastCGI server with the given config at `address` and returns a handle to it.
///
/// Binding to port `0` will request that the OS assign an available port.
///
/// If `address` yields multiple addresses, only the first one is considered.
///
/// This function does not block because the FastCGI server is created on a separate thread.
pub fn start(config: ServerConfig, address: impl ToSocketAddrs) -> Result<ServerHandle, io::Error> {
    let mut iter = address.to_socket_addrs()?;
    let first_address = iter
        .next()
        .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;
    event_loop::create_handle(config, first_address)
}
