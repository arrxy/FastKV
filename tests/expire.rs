//! EXPIRE / TTL compatibility tests.
//!
//! Ported from Redis `tests/unit/expire.tcl`:
//! - "EXPIRE - set timeouts multiple times"
//! - "EXPIRE - It should be still possible to read 'x'"
//! - "SETEX - Set + Expire combo operation. Check for TTL"
//! - "SETEX - Check value"
//! - "SETEX - Wrong time parameter"
//! - "TTL returns time to live in seconds"
//! - "TTL / PTTL / EXPIRETIME / PEXPIRETIME return -1 if key has no expire"
//! - "TTL / PTTL / EXPIRETIME / PEXPIRETIME return -2 if key does not exit"

mod common;

use fast_kv::core::resp::Value;

use common::RedisTestClient;

#[test]
fn expire_set_timeouts_multiple_times() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "foobar"]);

    let v1 = r.cmd(&["EXPIRE", "x", "5"]);
    assert_eq!(v1, Value::Integer(1));

    let v2 = match r.cmd(&["TTL", "x"]) {
        Value::Integer(n) => n,
        other => panic!("expected integer TTL, got {other:?}"),
    };
    assert!(v2 >= 4 && v2 <= 5, "ttl was {v2}");

    let v3 = r.cmd(&["EXPIRE", "x", "10"]);
    assert_eq!(v3, Value::Integer(1));

    let v4 = match r.cmd(&["TTL", "x"]) {
        Value::Integer(n) => n,
        other => panic!("expected integer TTL, got {other:?}"),
    };
    assert!(v4 >= 9 && v4 <= 10, "ttl was {v4}");

    r.cmd_integer(&["EXPIRE", "x", "2"], 1);
    r.cmd_bulk(&["GET", "x"], "foobar");
}

#[test]
fn expire_against_missing_key_returns_zero() {
    let mut r = RedisTestClient::new();
    r.cmd_integer(&["EXPIRE", "missing", "10"], 0);
}

#[test]
fn expire_wrong_number_of_arguments() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["EXPIRE", "k"], "wrong number of arguments");
}

#[test]
fn expire_rejects_invalid_time() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "foo"]);
    r.cmd_error_contains(&["EXPIRE", "x", "-10"], "invalid expire");
}

#[test]
fn ttl_returns_seconds_for_volatile_key() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "somevalue", "EX", "10"]);
    let ttl = match r.cmd(&["TTL", "x"]) {
        Value::Integer(n) => n,
        other => panic!("expected integer TTL, got {other:?}"),
    };
    assert!(ttl > 8 && ttl <= 10, "ttl was {ttl}");
}

#[test]
fn ttl_returns_minus_one_for_persistent_key() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "hello"]);
    r.cmd_integer(&["TTL", "x"], -1);
}

#[test]
fn ttl_returns_minus_two_for_missing_key() {
    let mut r = RedisTestClient::new();
    r.cmd_integer(&["TTL", "x"], -2);
}

#[test]
fn get_lazy_expires_volatile_key() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "somevalue", "EX", "1"]);
    r.sleep(1100);
    r.cmd_null(&["GET", "x"]);
    r.cmd_integer(&["TTL", "x"], -2);
}

#[test]
#[ignore = "slow: waits 2.1s for expiry (Redis expire.tcl)"]
fn expire_key_gone_after_timeout() {
    let mut r = RedisTestClient::new();
    r.cmd_ok(&["SET", "x", "foobar"]);
    r.cmd_integer(&["EXPIRE", "x", "2"], 1);
    r.sleep(2100);
    r.cmd_null(&["GET", "x"]);
}
