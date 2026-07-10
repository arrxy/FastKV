//! SET / GET compatibility tests.
//!
//! Ported from Redis `tests/unit/type/string.tcl`:
//! - "SET and GET an item"
//! - "SET and GET an empty item"
//! - "Zero length value in key. SET/GET/EXISTS" (keyspace.tcl, GET portion)
//! - "Extended SET EX option"
//! - "Extended SET PX option"

mod common;

use common::RedisTestClient;

#[test]
fn set_and_get_an_item() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "foobar"]);
    r.cmd_bulk(&["GET", "x"], "foobar");
}

#[test]
fn set_and_get_an_empty_item() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", ""]);
    r.cmd_bulk(&["GET", "x"], "");
}

#[test]
fn get_missing_key_returns_null() {
    let mut r = RedisTestClient::new();
    r.cmd_null(&["GET", "missing"]);
}

#[test]
fn get_wrong_number_of_arguments() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["GET"], "wrong number of arguments");
}

#[test]
fn set_wrong_number_of_arguments() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["SET", "k"], "wrong number of arguments");
    r.cmd_error_contains(&["SET", "k", "v", "EX"], "wrong number of arguments");
}

#[test]
fn set_ex_sets_ttl_in_seconds() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "foo", "bar", "EX", "10"]);
    let ttl = match r.cmd(&["TTL", "foo"]) {
        fast_kv::core::resp::Value::Integer(n) => n,
        other => panic!("expected integer TTL, got {other:?}"),
    };
    assert!(ttl > 5 && ttl <= 10, "ttl was {ttl}");
}

#[test]
fn set_px_sets_ttl_in_seconds() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "foo", "bar", "PX", "10000"]);
    let ttl = match r.cmd(&["TTL", "foo"]) {
        fast_kv::core::resp::Value::Integer(n) => n,
        other => panic!("expected integer TTL, got {other:?}"),
    };
    assert!(ttl > 5 && ttl <= 10, "ttl was {ttl}");
}

#[test]
fn set_ex_rejects_invalid_expire_time() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["SET", "z", "foo", "EX", "-10"], "invalid expire");
}

#[test]
fn set_ex_rejects_non_integer_expire() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["SET", "z", "foo", "EX", "abc"], "not an integer");
}

#[test]
fn set_rejects_unknown_expiry_unit() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["SET", "k", "v", "NX", "10"], "syntax error");
}

#[test]
fn overwrite_existing_key() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "k", "old"]);
    r.cmd_ok(&["SET", "k", "new"]);
    r.cmd_bulk(&["GET", "k"], "new");
}

#[test]
#[ignore = "slow: waits for key expiry (Redis expire.tcl SETEX wait test)"]
fn set_ex_key_expires() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "y", "foo", "EX", "1"]);
    r.cmd_bulk(&["GET", "y"], "foo");
    r.sleep(1100);
    r.cmd_null(&["GET", "y"]);
}
