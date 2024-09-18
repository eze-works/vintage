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
//!      let server = start("localhost:0", |ctx| {
//!          Some(ctx.with_body("<h1>Hello World</h1>"))
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
//!    Perhapse surprisingly, the return type of this function is `Option<FcgiContext>` instead of
//!    just [`FcgiContext`].
//!    This is intentional and fits nicely with the rest of the library.
//!    Returning `None` causes the server to respond with an empty 404.
//!
//! For less trivial request handling, this crate offers [`pipe`]s, a combinatorial approach to
//! chaining "middleware" together.
//!
//! In the following example, we setup a static file server for paths begining with `/assets`,
//! and a router for the `/about` path.
//! We then combine them using `or`, which yields a pipeline that tries to resolve the request path as
//! a static file.
//! If that fails, it will try finding a relevant callback using the router.
//! If that fails, the server just returns a 404 because the result of the pipeline will be `None`.
//!
//!
//! ```
//! use vintage::start;
//! use vintage::pipe::{self, Pipe};
//!
//! let router = pipe::Router::new().get(["/about"], |ctx, _| ctx);
//! let static_files = pipe::FileServer::new("/assets", "/var/www");
//! let pipeline = static_files.or(router);
//!
//! let server = start("localhost:0", move |ctx| pipeline.run(ctx)).unwrap();
//! server.stop();
//! ```
//!
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
mod context;
mod error;
mod fcgi_context;
mod record;
mod server;
mod server_spec;
pub mod status;

pub use context::{Request, Response};
pub use fcgi_context::FcgiContext;
pub use server::{ServerExitReason, ServerHandle};
pub use server_spec::ServerSpec;
