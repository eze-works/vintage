use mio::event::Source;
use mio::event::{Event, Events};
use mio::{Interest, Poll, Token, Waker};
use std::collections::HashMap;
use std::io;

#[derive(Debug, thiserror::Error)]
pub enum PollerError {
    #[error("io operation failed")]
    Io(#[from] io::Error),
}
// A more convenient callback-based event loop on top of mio
pub struct Poller {
    token_count: usize,
    poll: Poll,
    events: Events,
    handlers: HashMap<Token, Box<dyn FnMut(&Event) -> Result<(), io::Error>>>,
}

impl Poller {
    pub fn new() -> Result<Self, io::Error> {
        let poll = Poll::new()?;
        let events = Events::with_capacity(128);
        Ok(Poller {
            token_count: 0,
            poll,
            events,
            handlers: HashMap::new(),
        })
    }

    pub fn register<S, C>(&mut self, source: &mut S, callback: C) -> Result<(), io::Error>
    where
        S: Source,
        C: 'static,
        C: FnMut(&Event) -> Result<(), io::Error>,
    {
        let token = Token(self.token_count);
        self.token_count += 1;
        self.poll
            .registry()
            .register(source, token, Interest::READABLE | Interest::WRITABLE)?;
        let callback = Box::new(callback);

        self.handlers.insert(token, callback);
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), io::Error> {
        loop {
            self.poll.poll(&mut self.events, None)?;

            for event in self.events.iter() {
                let token = event.token();
                let Some(handler) = self.handlers.get_mut(&token) else {
                    unreachable!("unregistered token emitted: {token:?}");
                };

                loop {
                    match handler(event) {
                        Ok(_) => {}
                        Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                        Err(err) => return Err(err),
                    }
                }
            }
        }
    }
}
