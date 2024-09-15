//! A library for writing FastCGI application servers.
//! It mostly implements the [FastCGI](https://www.mit.edu/~yandros/doc/specs/fcgi-spec.html#S4)
//! spec, but [deviates](crate#deviations-from-the-spec) where it makes sense to do so.
//!
//! The [terminology](crate#terminology) section contains definitions for words used throughout the
//! documentation.
//!
//! Using this crate is straightforward:
//!
//!  ```
//!  use vintage::start;
//!
//!  fn main() {
//!      let server = start("localhost:8000", |ctx| {
//!          ctx.with_body("<h1>Hello World</h1>")
//!      }).unwrap();
//!      
//!      // This would block the current thread until the server thread exits
//!      // server.join()
//!
//!      // Gracefull shutdown
//!      server.stop();
//!  }
//!  ```
//!
//!  The [`start`] function accepts two arguments:
//!  - The address on which the server should listen.
//!  - A function to handle FastCGI requests.
//!    It is passed a single [`FcgiContext`] argument, and must return an value of the same type.
//!
//! The crate also provides an optional layer called [`pipe`]s to help compose request processing
//! together.
//!
//! # Terminology:
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

#![allow(dead_code)]
mod connection;
mod error;
mod fcgi_context;
pub mod pipe;
mod record;
mod server;
pub mod status;

pub use fcgi_context::FcgiContext;
pub use server::{start, ServerExitReason, ServerHandle};
