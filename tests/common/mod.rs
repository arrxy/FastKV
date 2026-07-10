//! Shared harness for Redis compatibility tests.
//!
//! Tests run commands through `RespCommandProcessor` over a loopback TCP pair,
//! matching the production request path without binding a real port.

use std::io::Read;
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::Duration;

use fast_kv::core::processor::RespCommandProcessor;
use fast_kv::core::resp::{self, Value};
use fast_kv::protocol::CommandProcessor;

pub struct RedisTestClient {
    processor: RespCommandProcessor,
    server: TcpStream,
    client: TcpStream,
}

#[allow(dead_code)]
impl RedisTestClient {
    pub fn new() -> Self {
        let (client, server) = loopback();
        Self {
            processor: RespCommandProcessor::new(),
            server,
            client,
        }
    }

    pub fn cmd(&mut self, parts: &[&str]) -> Value {
        let request = encode_cmd(parts);
        self.processor
            .process(&request, &mut self.server)
            .expect("command should not close connection");
        read_response(&mut self.client)
    }

    pub fn cmd_raw(&mut self, raw: &[u8]) -> Value {
        self.processor
            .process(raw, &mut self.server)
            .expect("command should not close connection");
        read_response(&mut self.client)
    }

    pub fn cmd_ok(&mut self, parts: &[&str]) {
        assert_eq!(self.cmd(parts), Value::SimpleString("OK".to_string()));
    }

    pub fn cmd_bulk(&mut self, parts: &[&str], expected: &str) {
        assert_eq!(
            self.cmd(parts),
            Value::BulkString(expected.as_bytes().to_vec())
        );
    }

    pub fn cmd_null(&mut self, parts: &[&str]) {
        assert_eq!(self.cmd(parts), Value::Null);
    }

    pub fn cmd_integer(&mut self, parts: &[&str], expected: i64) {
        assert_eq!(self.cmd(parts), Value::Integer(expected));
    }

    pub fn cmd_error_contains(&mut self, parts: &[&str], needle: &str) {
        match self.cmd(parts) {
            Value::Error(msg) => assert!(
                msg.contains(needle),
                "expected error containing '{needle}', got '{msg}'"
            ),
            other => panic!("expected error containing '{needle}', got {other:?}"),
        }
    }

    pub fn sleep(&self, ms: u64) {
        thread::sleep(Duration::from_millis(ms));
    }
}

fn loopback() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let client = TcpStream::connect(addr).unwrap();
    let (server, _) = listener.accept().unwrap();
    (client, server)
}

fn encode_cmd(parts: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", parts.len());
    for part in parts {
        out.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
    }
    out.into_bytes()
}

fn read_response(stream: &mut TcpStream) -> Value {
    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).expect("failed to read response");
    resp::decode(&buf[..n]).expect("failed to decode response")
}
