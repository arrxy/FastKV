# fast_kv Redis Compatibility Test Suite

Tests in `tests/` are ported from the [official Redis test suite](https://github.com/redis/redis/tree/unstable/tests) (Tcl). Each file maps to Redis upstream sources and covers only commands fast_kv implements today.

## Running tests

```bash
# All tests (fast unit + integration)
cargo test

# Integration tests only
cargo test --test '*'

# Skip slow expiry tests (marked #[ignore])
cargo test -- --ignored

# Run everything including slow tests
cargo test -- --include-ignored
```

## Test harness

`tests/common/mod.rs` provides `RedisTestClient`, which drives `RespCommandProcessor` over a loopback TCP pair — the same eval path as the live server, without binding a port.

## Source mapping

| fast_kv test file | Redis upstream source | Tests ported |
|-------------------|----------------------|--------------|
| `tests/ping.rs` | `tests/unit/protocol.tcl` | PONG, message echo, arity errors |
| `tests/echo.rs` | (standard command contract) | ECHO message, arity errors |
| `tests/string.rs` | `tests/unit/type/string.tcl` | SET/GET, empty values, SET EX/PX, syntax errors |
| `tests/del.rs` | `tests/unit/keyspace.tcl` | single/vararg DEL, missing keys, zero-length values |
| `tests/expire.rs` | `tests/unit/expire.tcl` | EXPIRE, TTL -1/-2, lazy expiry, SET EX |
| `tests/protocol.rs` | `tests/unit/protocol.tcl`, `keyspace.tcl` | bulk replies, pipelined command sequence |

## Not yet ported

Redis tests for unimplemented commands (MGET, SETNX, PERSIST, PEXPIRE, replication, cluster, etc.) are intentionally omitted. Add cases here as fast_kv gains parity.

## Slow tests

Some Redis tests wait for real-time expiry (`tags {"slow"}` in Tcl). Rust equivalents use `#[ignore]`:

- `string::set_ex_key_expires`
- `del::del_against_expired_key`
- `expire::expire_key_gone_after_timeout`

Run with `cargo test -- --ignored` when validating TTL behavior.
