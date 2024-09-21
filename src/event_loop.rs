use crate::connection::Connection;
use crate::fastcgi_responder;
use crate::server_config::ServerConfig;
use crate::server_handle::{ServerExitReason, ServerHandle};
use mio::event::Events;
use mio::net::TcpListener;
use mio::{Interest, Poll, Token, Waker};
use std::io;
use std::net::SocketAddr;
use std::sync::mpsc::{sync_channel, SyncSender};
use std::thread;

// Tokens used for the MIO event loop
const SERVER: Token = Token(0);
const SHUTDOWN: Token = Token(1);

struct EventLoop {
    socket: TcpListener,
    spec: ServerConfig,
    poll: Poll,
    events: Events,
    signal_shutdown: SyncSender<()>,
}

pub fn create_handle(spec: ServerConfig, address: SocketAddr) -> Result<ServerHandle, io::Error> {
    // One of the requirements is that the user of the library be able to shutdown the server
    // gracefully. This means that there should be some way for the user to say "finish all
    // in-flight work, then stop the thread pool".
    //
    // This requirement drastically changes how `ServerConfig::start()` works:
    // 1) It needs to return some type of handle the user can use to later stop the server.
    // 2) The handle needs to somehow "wake up" the call to `socket.accept()` when it is time to
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

    let mut socket = TcpListener::bind(address)?;

    let address = socket.local_addr()?;

    log::info!("FastCGI Server listening on {address}");

    let poll = Poll::new()?;

    let events = Events::with_capacity(128);

    let server_waker = Waker::new(poll.registry(), SHUTDOWN)?;

    poll.registry()
        .register(&mut socket, SERVER, Interest::READABLE)?;

    let (signal_shutdown, observe_shutdown) = sync_channel(0);

    let event_loop = EventLoop {
        socket,
        spec,
        poll,
        events,
        signal_shutdown,
    };

    let handle = thread::spawn(move || start(event_loop));

    Ok(ServerHandle {
        address,
        server_loop: handle,
        server_waker,
        observe_shutdown,
    })
}

fn start(mut evloop: EventLoop) -> ServerExitReason {
    // `shutdown_threadpool` should always be called before exiting this function, regardless of
    // cause.
    // This will ensure active threads finish their work.
    let pool = threadpool::Builder::new().build();

    loop {
        match evloop.poll.poll(&mut evloop.events, None) {
            Ok(_) => {}
            Err(err) => {
                log::warn!(error:err = err; "Poll call failed. Server loop will exit");
                shutdown_threadpool(pool);
                return ServerExitReason::Err(err);
            }
        };

        for event in evloop.events.iter() {
            match event.token() {
                SERVER => loop {
                    match evloop.socket.accept() {
                        Ok((stream, _)) => {
                            let connection = match Connection::try_from(stream) {
                                Ok(c) => c,
                                Err(err) => return ServerExitReason::Err(err),
                            };
                            pool.execute({
                                let spec = evloop.spec.clone();
                                move || {
                                    fastcgi_responder::handle_connection(connection, spec);
                                }
                            });
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => break,
                        Err(err) => {
                            log::warn!(error:err = err; "Socket accept call failed. Server loop will exit");
                            shutdown_threadpool(pool);
                            return ServerExitReason::Err(err);
                        }
                    }
                },
                SHUTDOWN => {
                    shutdown_threadpool(pool);
                    if evloop.signal_shutdown.send(()).is_err() {
                        // The only way this happens is if the main thread called
                        // `Server::server_waker.wake()` then immediately dropped
                        // the `Server::observe_shutdown` receiver such that this fails to
                        // send.
                        //
                        // But that cannot be, since we don't do that ... and those properties
                        // are not part of the public API.
                        //
                        // That said if somehow, it does happen, I do still want to know
                        log::error!(
                            "unreachable code reached! failed to notify main thread of shutdown."
                        );
                        unreachable!("failed to notify main thread of shutdown");
                    }
                    return ServerExitReason::Normal;
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
