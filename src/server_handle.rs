use std::io;
use std::net::SocketAddr;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// The reason the server exited
#[derive(Debug, Default)]
pub enum ServerExitReason {
    /// It was gracefully shutdown
    #[default]
    Normal,
    /// Polling the server socket for new connections failed somehow.
    Err(io::Error),
    /// The server panicked. The payload will contain the panic message.
    Panic(String),
}

/// Handle to a running FastCGI server
pub struct ServerHandle {
    pub(crate) address: SocketAddr,
    pub(crate) server_loop: JoinHandle<ServerExitReason>,
    pub(crate) server_waker: mio::Waker,
    pub(crate) observe_shutdown: Receiver<()>,
}

impl ServerHandle {
    /// Blocks until the server terminates and returns the reason.
    ///
    /// This function does not attempt to stop the server.
    /// It waits (potentially indefinitely) until the server exits.
    /// If you want to stop sthe server, use [`stop()`](crate::ServerHandle::stop).
    pub fn join(self) -> ServerExitReason {
        match self.server_loop.join() {
            Ok(r) => r,
            Err(any) => match any.as_ref().downcast_ref::<String>() {
                Some(s) => ServerExitReason::Panic(s.clone()),
                None => match any.as_ref().downcast_ref::<&str>() {
                    Some(s) => ServerExitReason::Panic(s.to_string()),
                    None => ServerExitReason::Panic(String::new()),
                },
            },
        }
    }

    /// Stops the FastCGI server
    ///
    /// The server waits for all in-flight requests to complete before it is shutdown
    pub fn stop(self) {
        // Wake up the server thread.
        // It will be able to tell that it was woken up by the waker instead of by a new readable Tcp connection.
        // If this call fails, just return.
        // We don't want to attempt to block on the `recv()` call in the next line if its possible
        // we didn't wake the server.
        // This means our graceful shutdown is "best effort".
        // Nothing we can do if some OS-level error happened.
        let Ok(()) = self.server_waker.wake() else {
            return;
        };

        // Normally, after the server thread is woken up by the waker, it will eventually
        // rendezvous here.
        // Except if it exited due to an error or panicked, in which case this call would return
        // with an error. But we ignore it because we only care that the server loop is stopped.
        let _ = self.observe_shutdown.recv();
    }

    /// Returns the address at which the server is currently listening
    pub fn address(&self) -> SocketAddr {
        self.address
    }
}
