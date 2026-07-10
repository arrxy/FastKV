//! ECHO compatibility tests.
//!
//! Redis has no dedicated ECHO test file; behavior follows the standard
//! command contract exercised by redis-cli and client libraries.

mod common;

use common::RedisTestClient;

#[test]
fn echo_returns_message() {
    let mut r = RedisTestClient::new();
    r.cmd_bulk(&["ECHO", "fast_kv"], "fast_kv");
}

#[test]
fn echo_with_empty_string() {
    let mut r = RedisTestClient::new();
    r.cmd_bulk(&["ECHO", ""], "");
}

#[test]
fn echo_without_arguments_returns_error() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["ECHO"], "wrong number of arguments");
}

#[test]
fn echo_with_too_many_arguments_returns_error() {
    let mut r = RedisTestClient::new();
    r.cmd_error_contains(&["ECHO", "a", "b"], "wrong number of arguments");
}
