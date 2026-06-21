use std::{io, os::fd::RawFd};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]

#[allow(dead_code)]
pub enum EventKind {
    Server,
    Client,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct EventObject {
    pub fd: RawFd,
    pub kind: EventKind,
}

#[allow(dead_code)]
impl EventObject {
    pub fn server(fd: RawFd) -> Self {
        Self {
            fd,
            kind: EventKind::Server,
        }
    }

    pub fn client(fd: RawFd) -> Self {
        Self {
            fd,
            kind: EventKind::Client,
        }
    }

    pub fn encode(self) -> usize {
        let tag = match self.kind {
            EventKind::Server => 1 as usize,
            EventKind::Client => 2 as usize,
        };
        ((self.fd as usize) << 8) | tag
    }

    pub fn decode(data: usize) -> Self {
        let tag = data & 0xff;
        let fd = (data >> 8) as RawFd;
        let kind = match tag {
            1 => EventKind::Server,
            2 => EventKind::Client,
            _ => panic!("unknown event kind"),
        };
        Self { fd, kind }
    }
}

#[allow(dead_code)]
pub trait Poller {
    fn new() -> io::Result<Self>
    where
        Self: Sized;
    fn add(&self, event: EventObject) -> io::Result<()>;
    fn delete(&self, fd: RawFd) -> io::Result<()>;
    fn wait(&mut self, timeout_ms: i32) -> io::Result<Vec<EventObject>>;
}

#[allow(dead_code)]
#[cfg(target_os = "linux")]
mod epoll;

#[allow(dead_code)]
#[cfg(target_os = "macos")]
mod kqueue;

#[allow(dead_code)]
#[cfg(target_os = "linux")]
pub use epoll::OsPoller;

#[allow(dead_code)]
#[allow(unused_imports)]
#[cfg(target_os = "macos")]
pub use kqueue::OsPoller;
