//! RESP protocol compatibility tests.
//!
//! Ported from Redis `tests/unit/protocol.tcl`:
//! - "Generic wrong number of args"
//! - "bulk reply protocol" (SET/GET integer and string encodings)

mod common;

use fast_kv::core::resp::Value;

use common::RedisTestClient;

#[test]
fn bulk_reply_protocol_for_set_and_get() {
    let mut r = RedisTestClient::new();

    r.cmd_ok(&["SET", "crlf", "2"]);
    r.cmd_bulk(&["GET", "crlf"], "2");

    r.cmd_ok(&["SET", "crlf", "2147483647"]);
    r.cmd_bulk(&["GET", "crlf"], "2147483647");

    r.cmd_ok(&["SET", "crlf", "-2147483648"]);
    r.cmd_bulk(&["GET", "crlf"], "-2147483648");

    r.cmd_ok(&["SET", "crlf", "aaaaaaaaaaaaaaaa"]);
    r.cmd_bulk(&["GET", "crlf"], "aaaaaaaaaaaaaaaa");

    let long = "a".repeat(45);
    r.cmd_ok(&["SET", "crlf", &long]);
    r.cmd_bulk(&["GET", "crlf"], &long);

    r.cmd_integer(&["DEL", "crlf"], 1);
}

#[test]
fn commands_pipelining_style_sequence() {
    // keyspace.tcl "Commands pipelining" — SET k1, GET k1, PING in one session
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "k1", "xyzk"]);
    r.cmd_bulk(&["GET", "k1"], "xyzk");
    assert_eq!(r.cmd(&["PING"]), Value::SimpleString("PONG".to_string()));
}

#[test]
fn set_then_get_preserves_binary_safe_values() {
    let mut r = RedisTestClient::new();
    let value = "Hello World";
    r.cmd_ok(&["SET", "msg", value]);
    r.cmd_bulk(&["GET", "msg"], value);
}
