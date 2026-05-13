# wtrs

A safe, type-safe Rust wrapper for [WiredTiger](https://source.wiredtiger.com/), the high-performance embedded database engine used as MongoDB's storage engine.

## Crates

| Crate | Description |
|-------|-------------|
| `wtrs` | Safe, ergonomic Rust API over WiredTiger |
| `wt-sys` | Low-level FFI bindings; compiles WiredTiger C from source |
| `ext_crypto` | Example WiredTiger extension (`cdylib`) |

WiredTiger itself is a git submodule at `wt-sys/src/wiredtiger` and is compiled from source at build time.

## Prerequisites

- Rust toolchain (stable)
- A C compiler (clang or gcc)
- Git submodules initialized

```bash
git submodule update --init --recursive
```

## Build

```bash
cargo build
cargo build --release
```

## Test

```bash
cargo test
```

## Examples

### ex_hello — open a connection and session

```bash
cargo run --example ex_hello
```

Opens a WiredTiger database, creates a session, and closes cleanly. Rust port of `wiredtiger/examples/c/ex_hello.c`.

### ex_cursor — cursor-based key/value operations

```bash
cargo run --example ex_cursor
```

Creates a string-keyed table, inserts/updates/deletes records, and scans results forward. Rust port of `wiredtiger/examples/c/ex_cursor.c`.

## Type Hierarchy

WiredTiger's resource model maps to Rust lifetimes:

```
Connection  (Send + Sync)
  └── Session<'conn>  (Send)
        └── Cursor<'session>  (Send)
```

- **`Connection`** — opened via `Connection::open(path, config)`; auto-closes on drop.
- **`Session`** — created via `conn.open_session(config)`; manages transactions.
- **`Cursor`** — created via `session.open_cursor(uri, config)`; used for all data access and mutation.

## Extensions

WiredTiger extensions are `cdylib` crates that export `wiredtiger_extension_init()`. The `ext_crypto` crate demonstrates this pattern.

```bash
cargo build -p ext_crypto
```
