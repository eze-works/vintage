use crate::connection::Connection;
use crate::error::Error;
use crate::record::*;
use crate::request::Request;
use crate::response::Response;
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
    // + We receive a `BeginRequest` request followed by Params and Stdin. Respond using Stdout followed by EndRequest
    //
    // What about the AbortRequest message you ask?
    // We are not multiplexing connections, so the client can abort requests by closing the connection.
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

        let stdout = Stdout::from(response);
        conn.write_record(&Record::Stdout(stdout)).unwrap();
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

fn respond_with_values(conn: &mut Connection, record: GetValues) {
    for variable in record.get_variables() {
        // If the client cares, tell it we do not want to multiplex connections
        if variable == "FCGI_MPXS_CONNS" {
            let response = GetValuesResult::new([("FCGI_MPXS_CONNS", "0")]);
            let _ = conn.write_record(&Record::GetValuesResult(response));
            break;
        }
    }
}
