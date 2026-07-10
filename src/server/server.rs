use crate::config::config::Config;
use crate::protocol::CommandProcessor;
use crate::rk_info;
use pollio::{EventKind, EventObject, OsPoller, Poller};

use std::collections::HashMap;
use std::io::{ErrorKind, prelude::*};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::io::{AsRawFd, RawFd};
use std::time::{SystemTime, UNIX_EPOCH};

// A client with this much unparseable input pending is broken or abusive.
const MAX_INPUT_BUFFER: usize = 64 * 1024 * 1024;

struct Connection {
    stream: TcpStream,
    // bytes read but not yet parsed as a complete command
    input: Vec<u8>,
}

pub struct Server<P: CommandProcessor> {
    listener: TcpListener,
    poller: OsPoller,
    connections: HashMap<RawFd, Connection>,
    con_clients: u64,
    events_buf: Vec<EventObject>,
    out_buf: Vec<u8>,
    processor: P,
    last_cleanup_time: u128,
    cleanup_interval: u128,
}

impl<P: CommandProcessor> Server<P> {
    pub fn new(processor: P) -> Self {
        let (listener, poller, cleanup_interval) = Self::boot_up_server().unwrap();
        Self {
            listener,
            poller,
            connections: HashMap::new(),
            con_clients: 0,
            events_buf: Vec::new(),
            out_buf: Vec::new(),
            processor,
            last_cleanup_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis(),
            cleanup_interval,
        }
    }

    pub fn run(&mut self) -> Result<(), std::io::Error> {
        loop {
            if SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis()
                - self.last_cleanup_time
                >= self.cleanup_interval
            {
                self.processor.cleanup_expired_keys()?;
                self.last_cleanup_time = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis();
            }
            match self.poller.wait(self.cleanup_interval as i32) {
                Ok(events) => {
                    self.events_buf.clear();
                    self.events_buf.extend_from_slice(events);
                }

                Err(e) if e.kind() == ErrorKind::Interrupted => {
                    continue;
                }

                Err(_e) => {
                    continue;
                }
            }

            for i in 0..self.events_buf.len() {
                let event = self.events_buf[i];
                match event.kind {
                    EventKind::Server => self.handle_server_events()?,
                    EventKind::Client => self.handle_client_events(&event)?,
                }
            }
        }
    }

    fn close_client(&mut self, fd: RawFd) {
        if let Err(e) = self.poller.delete(fd) {
            rk_info!("[CLOSE] failed to delete fd {} from poller: {}", fd, e);
        }

        if let Some(conn) = self.connections.remove(&fd) {
            let _ = conn.stream.shutdown(Shutdown::Both);
        }

        self.con_clients = self.con_clients.saturating_sub(1);
    }

    fn boot_up_server() -> Result<(TcpListener, OsPoller, u128), std::io::Error> {
        let config: Config = Config::new();
        let address = format!("{}:{}", config.get_host(), config.get_port());
        let cleanup_interval = config.get_cleanup_interval();

        rk_info!("[BOOT] server starting \n address = {}", address);
        let listener: TcpListener = TcpListener::bind(&address)?;
        rk_info!(
            "[BOOT] listener bound successfully local addr = {}",
            listener.local_addr()?
        );

        listener.set_nonblocking(true)?;
        let listener_fd = listener.as_raw_fd();
        rk_info!("[BOOT] listener fd = {}", listener_fd);
        let poller = OsPoller::new()?;
        poller.add(EventObject::server(listener_fd))?;

        Ok((listener, poller, cleanup_interval))
    }

    fn handle_server_events(&mut self) -> Result<(), std::io::Error> {
        loop {
            match self.listener.accept() {
                Ok((stream, _address)) => {
                    stream.set_nonblocking(true)?;
                    stream.set_nodelay(true)?;
                    let client_fd = stream.as_raw_fd();
                    self.poller.add(EventObject::client(client_fd))?;
                    self.connections.insert(
                        client_fd,
                        Connection {
                            stream,
                            input: Vec::new(),
                        },
                    );
                    self.con_clients += 1;
                }

                // If the listener is blocked, break the loop. exit #1: accept queue is empty
                Err(e) if e.kind() == ErrorKind::WouldBlock => {
                    break;
                }

                // exit #2: a real accept error
                Err(e) => {
                    rk_info!(
                        "[SERVER] error accepting connection: kind={:?}, err={}",
                        e.kind(),
                        e
                    );
                    break;
                }
            }
        }
        Ok(())
    }

    fn handle_client_events(&mut self, event: &EventObject) -> Result<(), std::io::Error> {
        let fd = event.fd;
        let Some(conn) = self.connections.get_mut(&fd) else {
            return Ok(());
        };

        // Drain the socket into the connection's input buffer.
        let mut peer_closed = false;
        let mut should_close = false;
        let mut buffer: [u8; 16384] = [0; 16384];
        loop {
            match conn.stream.read(&mut buffer) {
                Ok(0) => {
                    peer_closed = true;
                    break;
                }
                Ok(n) => {
                    conn.input.extend_from_slice(&buffer[..n]);
                    if n < buffer.len() {
                        break;
                    }
                }
                Err(e) if e.kind() == ErrorKind::WouldBlock => break,
                Err(e) => {
                    rk_info!(
                        "[CLIENT] error reading from fd {}: kind={:?}, err={}",
                        fd,
                        e.kind(),
                        e
                    );
                    should_close = true;
                    break;
                }
            }
        }

        // Execute every complete command; leftover bytes wait for the next read.
        self.out_buf.clear();
        if !should_close && !conn.input.is_empty() {
            match self.processor.process(&conn.input, &mut self.out_buf) {
                Ok(consumed) => {
                    conn.input.drain(..consumed);
                }
                Err(_) => should_close = true,
            }
        }

        // One write syscall for the whole pipeline of responses.
        if !should_close && !self.out_buf.is_empty() {
            should_close = write_all_blocking(&mut conn.stream, &self.out_buf).is_err();
        }

        if conn.input.len() > MAX_INPUT_BUFFER {
            should_close = true;
        }

        if should_close || peer_closed {
            self.close_client(fd);
        }
        Ok(())
    }
}

// on WouldBlock this spins until the client drains its socket,
// stalling the event loop; switch to per-connection output buffers +
// writable-interest if slow clients become a real workload.
fn write_all_blocking(stream: &mut TcpStream, buf: &[u8]) -> std::io::Result<()> {
    let mut written = 0;
    while written < buf.len() {
        match stream.write(&buf[written..]) {
            Ok(n) => written += n,
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                continue;
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}
