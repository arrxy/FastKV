# FastKV

A Redis-compatible in-memory data store built for live video conferencing — room state, presence, signaling, and session metadata with a familiar protocol and client ecosystem.

> **Status: Under active development.** The server currently accepts TCP connections and echoes input. RESP parsing and Redis commands are in progress.

## Why FastKV?

Video conferencing backends lean heavily on fast, ephemeral state: who is in a room, mute/camera flags, hand-raise queues, breakout assignments, and short-lived signaling payloads. Redis is a common choice, but general-purpose deployments don't always match the access patterns and operational needs of real-time rooms.

FastKV aims to be a **drop-in Redis replacement** tuned for that workload — same wire protocol and client libraries, semantics shaped around live sessions.

## Current capabilities

| Area | Status |
|------|--------|
| TCP server | Working — synchronous, single-threaded |
| Configuration | `HOST` / `PORT` via environment variables |
| RESP decoder | Partial — simple strings, errors, integers |
| Redis commands | Not yet implemented |
| Persistence / replication | Planned |

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) (2024 edition)

## Quick start

```bash
git clone <your-repo-url>
cd FastKV
cargo run
```

The server listens on `0.0.0.0:9736` by default.

### Configuration

Set environment variables before starting:

| Variable | Default | Description |
|----------|---------|-------------|
| `HOST` | `0.0.0.0` | Bind address |
| `PORT` | `9736` | Listen port |

Example:

```bash
HOST=127.0.0.1 PORT=6379 cargo run
```

### Try it

```bash
nc localhost 9736
```

Type any text and press Enter — the server echoes it back. Full RESP command handling is coming next.

## Project layout

```
src/
├── main.rs           # Entry point
├── config/           # Host/port configuration
├── server/           # TCP listener and client handling
└── core/
    └── resp.rs       # Redis Serialization Protocol (RESP) types and decoder
```

## Roadmap

- [ ] Complete RESP encode/decode (bulk strings, arrays)
- [ ] Core commands: `PING`, `GET`, `SET`, `DEL`, `EXPIRE`
- [ ] Room-oriented data structures (presence sets, session hashes)
- [ ] Async I/O and concurrent client handling
- [ ] Optional persistence and replication

## License

TBD
