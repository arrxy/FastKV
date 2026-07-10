//! DEL compatibility tests.
//!
//! Ported from Redis `tests/unit/keyspace.tcl`:
//! - "DEL against a single item"
//! - "Vararg DEL"
//! - "Zero length value in key. SET/GET/EXISTS" (DEL portion)

mod common;

use common::RedisTestClient;

#[test]
fn del_against_a_single_item() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "foo"]);
    r.cmd_bulk(&["GET", "x"], "foo");
    r.cmd_integer(&["DEL", "x"], 1);
    r.cmd_null(&["GET", "x"]);
}

#[test]
fn vararg_del() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "foo1", "a"]);
    r.cmd_ok(&["SET", "foo2", "b"]);
    r.cmd_ok(&["SET", "foo3", "c"]);
    r.cmd_integer(&["DEL", "foo1", "foo2", "foo3", "foo4"], 3);
    r.cmd_null(&["GET", "foo1"]);
    r.cmd_null(&["GET", "foo2"]);
    r.cmd_null(&["GET", "foo3"]);
}

#[test]
fn del_against_missing_key_returns_zero() {
    let mut r = RedisTestClient::new();
    r.cmd_integer(&["DEL", "nokey"], 0);
}

#[test]
fn del_without_arguments_returns_error() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["DEL"], "wrong number of arguments");
}

#[test]
fn del_zero_length_value() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "emptykey", ""]);
    r.cmd_bulk(&["GET", "emptykey"], "");
    r.cmd_integer(&["DEL", "emptykey"], 1);
    r.cmd_null(&["GET", "emptykey"]);
}

#[test]
#[ignore = "slow: waits for active expiry (Redis keyspace.tcl expired DEL test)"]
fn del_against_expired_key() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "keyExpire", "valExpire", "EX", "1"]);
    r.sleep(1100);
    r.cmd_integer(&["DEL", "keyExpire"], 0);
}
