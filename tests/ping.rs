//! PING compatibility tests.
//!
//! Ported from Redis `tests/unit/protocol.tcl`:
//! - "Generic wrong number of args" (ping x y z)
//! - implicit PONG checks used throughout the suite

mod common;

use fast_kv::core::resp::Value;

use common::RedisTestClient;

#[test]
fn ping_without_arguments_returns_pong() {
    let mut r = RedisTestClient::new();
    assert_eq!(r.cmd(&["PING"]), Value::SimpleString("PONG".to_string()));
}

#[test]
fn ping_with_message_returns_bulk_string() {
    let mut r = RedisTestClient::new();
    assert_eq!(
        r.cmd(&["PING", "hello world"]),
        Value::BulkString(b"hello world".to_vec())
    );
}

#[test]
fn ping_with_too_many_arguments_returns_error() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["PING", "x", "y", "z"], "wrong number of arguments");
}
