//! Shared harness for Redis compatibility tests.
//!
//! Tests run commands through `RespCommandProcessor`, matching the
//! production request path; responses are decoded from the output buffer.

use std::thread;
use std::time::Duration;

use fast_kv::core::processor::RespCommandProcessor;
use fast_kv::core::resp::{self, Value};
use fast_kv::protocol::CommandProcessor;

pub struct RedisTestClient {
    processor: RespCommandProcessor,
}

#[allow(dead_code)]
impl RedisTestClient {
    pub fn new() -> Self {
        Self {
            processor: RespCommandProcessor::new(),
        }
    }

    pub fn cmd(&mut self, parts: &[&str]) -> Value {
        self.cmd_raw(&encode_cmd(parts))
    }

    pub fn cmd_raw(&mut self, raw: &[u8]) -> Value {
        let mut out = Vec::new();
        self.processor
            .process(raw, &mut out)
            .expect("command should not close connection");
        resp::decode(&out).expect("failed to decode response")
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

fn encode_cmd(parts: &[&str]) -> Vec<u8> {
    let mut out = format!("*{}\r\n", parts.len());
    for part in parts {
        out.push_str(&format!("${}\r\n{}\r\n", part.len(), part));
    }
    out.into_bytes()
}
