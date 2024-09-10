use crate::connection::Connection;
use crate::error::Error;
use crate::record::*;
use std::collections::BTreeMap;
use std::io;
use std::net::{TcpListener, ToSocketAddrs};
use std::sync::Arc;

/// An empty enum. Used to document a bit more explicitely that [`Server::run`](crate::Server::run)
/// will never return under normal conditions.
pub enum NoReturn {}

/// A FastCGI server
pub struct Server<F> {
    pool: threadpool::ThreadPool,
    handler: Arc<F>,
    socket: TcpListener,
}

impl<F> Server<F>
where
    F: 'static + Send + Sync + Fn(Request) -> Response,
{
    /// Creates a new FastCGI server by binding to the specified address.
    ///
    /// # Errors
    ///
    /// Returns an error if a socket could not be bound
    /// (e.g. lack of permissions, the socket was already bound to some other process ...)
    pub fn new<A: ToSocketAddrs>(handler: F, addr: A) -> Result<Self, io::Error> {
        let socket = TcpListener::bind(addr)?;
        Ok(Self {
            pool: threadpool::Builder::new().build(),
            handler: Arc::new(handler),
            socket,
        })
    }

    /// Runs the server by looping forever while responding to connections.
    ///
    /// This function only returns if the socket is shutdown (presumably by the OS)
    pub fn run(&self) -> Result<NoReturn, io::Error> {
        println!("Started server...");
        loop {
            let (stream, _) = self.socket.accept()?;
            let connection = Connection::try_from(stream)?;
            let handler = self.handler.clone();
            self.pool.execute(move || {
                Self::fast_cgi(connection, handler);
            });
        }
    }

    // Handles a FastCGI Connection.
    //
    // There are two expected flows;
    // + We receive a `GetValues` request to which we respond.
    // + We receive a `BeginRequest` request followed by Params and Stdin. Package these up into a `Request` and run the handler
    fn fast_cgi(mut conn: Connection, handler: Arc<F>) {
        let first_record = match conn.read_record() {
            Ok(r) => r,
            Err(e) => {
                handle_error(&mut conn, e);
                return;
            }
        };

        if let Record::GetValues(r) = first_record {
            respond_with_values(&mut conn, r);
            return;
        }

        let Record::BeginRequest(begin) = first_record else {
            eprintln!("Unexpected first record");
            return;
        };

        if begin.keep_alive() {
            let response =
                Record::EndRequest(EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported));
            let _ = conn.write_record(&response);
            eprintln!("Keep alive is not supported");
            return;
        }

        let params = match conn.expect_params() {
            Ok(params) => params,
            Err(None) => {
                eprintln!("Expected Params");
                return;
            }
            Err(Some(e)) => {
                handle_error(&mut conn, e);
                return;
            }
        };

        let stdin = match conn.expect_stdin() {
            Ok(stdin) => stdin,
            Err(None) => {
                eprintln!("Expected Stdin");
                return;
            }
            Err(Some(e)) => {
                handle_error(&mut conn, e);
                return;
            }
        };

        let response = handler(Request {
            vars: params,
            body: stdin,
        });

        let stdout = response.stdout.unwrap_or_default();

        conn.write_record(&Record::Stdout(stdout)).unwrap();
        if let Some(stderr) = response.stderr {
            conn.write_record(&Record::Stderr(stderr)).unwrap();
        }

        conn.write_record(&Record::EndRequest(EndRequest::new(
            0,
            ProtocolStatus::RequestComplete,
        )))
        .unwrap();
    }
}

fn handle_error(conn: &mut Connection, e: Error) {
    match e {
        Error::UnsupportedRole(_) => {
            let response = Record::EndRequest(EndRequest::new(0, ProtocolStatus::UnknownRole));
            let _ = conn.write_record(&response);
        }
        Error::MultiplexingUnsupported => {
            let response =
                Record::EndRequest(EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported));
            let _ = conn.write_record(&response);
        }
        _ => {}
    }
}
fn expect_record(conn: &mut Connection) -> Option<Record> {
    match conn.read_record() {
        e @ Err(Error::UnsupportedRole(_)) => {
            eprintln!("{e:?}");
            let response = Record::EndRequest(EndRequest::new(0, ProtocolStatus::UnknownRole));
            let _ = conn.write_record(&response);
            None
        }
        e @ Err(Error::MultiplexingUnsupported) => {
            eprintln!("{e:?}");
            let response =
                Record::EndRequest(EndRequest::new(0, ProtocolStatus::MultiplexingUnsupported));
            let _ = conn.write_record(&response);
            None
        }
        Err(_) => None,
        Ok(record) => Some(record),
    }
}

fn respond_with_values(_conn: &mut Connection, _record: GetValues) {
    todo!()
}

pub struct Request {
    vars: Params,
    body: Stdin,
}

impl Request {
    /// Returns the value of the CGI meta-variable `name`, if it exists
    pub fn get(&self, name: &str) -> Option<&str> {
        self.vars.get(name)
    }

    /// Returns the body of the request.
    ///
    /// The returned `Ved` will be empty if the request had no body.
    /// If there was a body, note that subsequent invocations will return an empty `Vec`.
    pub fn read_body(&mut self) -> Vec<u8> {
        self.body.take()
    }
}

pub struct Response {
    stdout: Option<Stdout>,
    stderr: Option<Stderr>,
}

impl Response {
    /// Returns an empty FastCGI response
    pub fn new() -> Self {
        Self {
            stdout: None,
            stderr: None,
        }
    }

    /// Set the body for the FastCGI response
    pub fn set_body(&mut self, output: Vec<u8>) {
        self.stdout = Some(Stdout::new(output));
    }

    /// Set the errror stream of the FastCGI response
    ///
    /// Note: The contents of this stream are at best logged by the FastCGI client.
    /// At worst, they are ignored
    pub fn set_errors(&mut self, errors: Vec<u8>) {
        self.stderr = Some(Stderr::new(errors));
    }
}
