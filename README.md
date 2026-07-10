# fast_kv

A Redis-compatible in-memory data store — same RESP wire protocol and client ecosystem, built to be a faster, leaner alternative to Redis for key-value workloads.

> **Status: Under active development.** Core string commands, TTL/expiry, and eviction are implemented. Many Redis commands are still on the roadmap.

## Why fast_kv?

Redis is the default choice for in-memory caching and session state, but it carries years of general-purpose complexity. **fast_kv** keeps what matters — RESP compatibility, familiar commands, TTL semantics — and strips away what you don't need for a focused key-value server.

Use any Redis client. Run the same commands you already know. Get a server written in Rust with a small surface area and room to optimize.

## Current capabilities

| Area | Status |
|------|--------|
| TCP server | Working — async event loop (`pollio`), concurrent clients |
| RESP protocol | Encode/decode — simple strings, errors, integers, bulk strings, arrays, null |
| String commands | `PING`, `ECHO`, `SET`, `GET`, `DEL` |
| Expiry | `EXPIRE`, `TTL`, `SET EX` / `SET PX`, lazy + active expiry sweep |
| Eviction | Configurable policies (LRU, LFU, random) via `MAX_KEYS` + `EVICTION_POLICY` |
| Persistence / replication | Not yet |

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)

## Quick start

```bash
git clone <your-repo-url>
cd fast_kv
cargo run
```

The server listens on `0.0.0.0:9736` by default.

### Configuration

Set environment variables before starting (or use a `.env` file via `dotenvy`):

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `9736` | Listen port |
| `MAX_KEYS` | `1000` | Maximum keys before eviction kicks in |
| `EVICTION_POLICY` | `NoEviction` | `NoEviction`, `AllKeysLru`, `VolatileLru`, `AllKeysLfu`, `VolatileLfu`, `AllKeysRandom`, `VolatileRandom` |
| `EVICTION_SAMPLE_SIZE` | `20` | Sample size for LRU/LFU eviction |
| `CLEANUP_INTERVAL` | `1000` | Active expiry sweep interval (ms) |

Example:

```bash
HOST=127.0.0.1 PORT=6379 MAX_KEYS=10000 EVICTION_POLICY=AllKeysLru cargo run
```

### Try it

```bash
# Plain text (decoder accepts simple strings)
nc localhost 9736
PING

# Or RESP array commands
redis-cli -p 9736 SET mykey hello
redis-cli -p 9736 GET mykey
```

## Testing

fast_kv ships with a Redis compatibility test suite — cases ported from the [official Redis test suite](https://github.com/redis/redis/tree/unstable/tests).

```bash
cargo test                  # unit + integration (fast)
cargo test -- --ignored     # include slow expiry tests
```

See [tests/README.md](tests/README.md) for the full mapping to upstream Redis test files.

## Project layout

```
src/
├── main.rs              # Entry point
├── lib.rs               # Library crate (used by integration tests)
├── config/              # Environment-based configuration
├── server/              # Async TCP server (pollio event loop)
├── core/
│   ├── resp.rs          # RESP encode/decode
│   ├── cmd.rs           # Parsed command representation
│   ├── eval.rs          # Command dispatch + in-memory store
│   └── processor.rs     # Request pipeline
└── logger.rs            # File or stdout logging
tests/                   # Redis compatibility integration tests
```

## Roadmap

- [ ] More string commands (`MGET`, `SETNX`, `INCR`, …)
- [ ] Hash, list, and set data types
- [ ] `PERSIST`, `PEXPIRE`, `PTTL`
- [ ] Pipelining and partial-buffer reads
- [ ] Optional persistence (RDB/AOF)
- [ ] Replication

## License

TBD
