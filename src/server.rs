use crate::connection::Connection;
use crate::error::Error;
use crate::record::*;
use crate::request::Request;
use crate::response::Response;
use mio::event::Events;
use mio::net::TcpListener;
use mio::{Interest, Poll, Token, Waker};
use std::io;
use std::net::ToSocketAddrs;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;
use std::thread;

// TODO: Logger library
// Log everywhere you ignore errors.

/// Handle to a running FastCGI server
pub struct ServerHandle {
    server_waker: Waker,
    observe_shutdown: Receiver<()>,
}

struct Server<F> {
    socket: TcpListener,
    handler: Arc<F>,
    poll: Poll,
    events: Events,
    signal_shutdown: SyncSender<()>,
}

const SERVER: Token = Token(0);
const SHUTDOWN: Token = Token(1);

/// Starts a new FastCGI server bound to the specified address, and returns a handle.
///
/// This function does not block. The FastCGI server is created on a separate thread.
pub fn start<A, F>(addr: A, handler: F) -> Result<ServerHandle, io::Error>
where
    A: ToSocketAddrs,
    F: 'static + Send + Sync + Fn(Request) -> Response,
{
    // One of the requirements is that the user of the library be able to shutdown the server
    // gracefully. This means that there should be some way for the user to say "finish all
    // in-flight work, then stop the thread pool".
    //
    // This requirement drastically changes how `start()` works:
    // 1) It needs to return some type of handle the user can use to later stop it
    // 2) The handle needs to  somehow "wake up" the call to `socket.accept()` when it is time to
    //    shutdown.
    //
    // Point (2) can't be done with the standard library (at least currently).
    // See this relevant discussion:
    // https://users.rust-lang.org/t/how-to-properly-close-a-tcplistener-in-multi-thread-server/87376
    //
    // Enter mio.
    //
    // The server thread no longer revolves around the call to `socket.accept()`. It now blocks on
    // `mio::Poll::poll()`. Mio gives us tools to wake up from that call.
    //
    // This gives us a nice way to implement graceful shutdown:
    // 1) Wake up the server thread from the `poll()` call with a Waker.
    // 2) On the server thread, join the thread pool, and drop it.
    // 3) Use a bounded channel of size 0 to "rendezvous" the main thread and the server
    //    thread. (A bounded channel of size 0 acts as a barrier. But allows timeouts.)
    //
    // That said, working with mio requires some care.
    // Familiarize yourself with this section of its documentation as any comments that follow
    // assume a baseline understanding of the workflow:
    // https://docs.rs/mio/latest/mio/struct.Poll.html#portability

    let address = addr
        .to_socket_addrs()?
        .next()
        .ok_or(io::Error::from(io::ErrorKind::InvalidInput))?;

    let mut socket = TcpListener::bind(address)?;

    let poll = Poll::new()?;

    let events = Events::with_capacity(128);

    let server_waker = Waker::new(poll.registry(), SHUTDOWN)?;

    poll.registry()
        .register(&mut socket, SERVER, Interest::READABLE)?;

    let (signal_shutdown, observe_shutdown) = sync_channel(0);

    let server = Server {
        socket,
        handler: Arc::new(handler),
        poll,
        events,
        signal_shutdown,
    };

    thread::spawn(move || {
        let _ = server.server_loop();
    });

    Ok(ServerHandle {
        server_waker,
        observe_shutdown,
    })
}

impl ServerHandle {
    /// Stops the FastCGI server
    ///
    /// The server waits for all in-flight requests to complete before it is shutdown
    pub fn stop(self) {
        // Wake up the server thread. It will be able to tell that it was woken up by the waker
        // instead of by a new readable Tcp connection.
        // If this call fails, just return. We don't want to attempt to block on the `recv()` call
        // in the next line if its possible we didn't wake the server.
        // This means our graceful shutdown is "best effort". Nothing we can do if some OS-level
        // error happened.
        let Ok(()) = self.server_waker.wake() else {
            return;
        };

        // Normally, after the server thread is woken up by the waker, it will eventually
        // rendezvous here.
        // Except if it exited due to an error or panicked, in which case this call would return
        // with an error. But we ignore it because we only care that the server loop is stopped.
        let _ = self.observe_shutdown.recv();
    }
}

impl<F> Server<F>
where
    F: 'static + Send + Sync + Fn(Request) -> Response,
{
    fn server_loop(mut self) -> Result<(), io::Error> {
        // `shutdown_threadpool` should always be called before exiting this function, regardless of
        // cause.
        // This will ensure active threads finish their work.
        let pool = threadpool::Builder::new().build();

        loop {
            match self.poll.poll(&mut self.events, None) {
                Ok(_) => {}
                Err(err) => {
                    Self::shutdown_threadpool(pool);
                    return Err(err);
                }
            };

            for event in self.events.iter() {
                match event.token() {
                    SERVER => loop {
                        match self.socket.accept() {
                            Ok((stream, _)) => {
                                let connection = Connection::from(stream);
                                let handler = self.handler.clone();
                                pool.execute(move || {
                                    Self::fast_cgi(connection, handler);
                                });
                            }
                            Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                            Err(err) => {
                                Self::shutdown_threadpool(pool);
                                return Err(err);
                            }
                        }
                    },
                    SHUTDOWN => {
                        Self::shutdown_threadpool(pool);
                        let _ = self.signal_shutdown.send(());
                        return Ok(());
                    }
                    _ => unreachable!(),
                }
            }
        }
    }

    fn shutdown_threadpool(pool: threadpool::ThreadPool) {
        pool.join();
        drop(pool);
    }

    // Handles a FastCGI Connection.
    //
    // There are two expected flows;
    // + We receive a `GetValues` request to which we respond.
    // + We receive a `BeginRequest` request followed by Params and Stdin. Respond using Stdout followed by EndRequest
    //
    // What about the AbortRequest message you ask?
    // We are not multiplexing connections, so the client can abort requests by closing the connection.
    fn fast_cgi(mut conn: Connection, handler: Arc<F>)
    where
        F: 'static + Send + Sync + Fn(Request) -> Response,
    {
        let first_record = match conn.read_record() {
            Ok(r) => r,
            Err(e) => {
                Self::handle_error(&mut conn, e);
                return;
            }
        };

        if let Record::GetValues(r) = first_record {
            Self::respond_with_values(&mut conn, r);
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
                Self::handle_error(&mut conn, e);
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
                Self::handle_error(&mut conn, e);
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
}
