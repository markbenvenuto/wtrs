# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Project Is

**wtrs** is a safe, type-safe Rust wrapper library for [WiredTiger](https://source.wiredtiger.com/), a high-performance embedded database engine (the storage engine used by MongoDB). It provides:
- `wt-sys`: low-level FFI bindings generated via `bindgen` from WiredTiger's C headers
- `wtrs` (root crate): ergonomic, type-safe Rust API over `wt-sys`
- `ext_crypto`: example of building a WiredTiger extension as a `cdylib`

WiredTiger itself lives as a git submodule at `wt-sys/src/wiredtiger` and is compiled from source at build time by `wt-sys/build.rs`.

## Commands

```bash
# Build
cargo build
cargo build --release

# Test
cargo test

# Run examples
cargo run --example ex_hello
cargo run --example ex_cursor

# Build the crypto extension (cdylib)
cargo build -p ext_crypto
```

## Architecture

### Workspace Structure

```
wtrs/           ← main crate: safe Rust API
wt-sys/         ← FFI bindings crate; compiles WiredTiger C from source
ext_crypto/     ← example extension (cdylib)
examples/       ← runnable examples using the wtrs API
```

### Core Type Hierarchy (src/lib.rs)

WiredTiger's resource model maps to Rust lifetimes:

```
Connection  (Send + Sync)
  └── Session<'conn>  (Send, borrows Connection)
        └── Cursor<'session>  (Send, borrows Session)
```

- **`Connection`** — opened via `Connection::open(path, config)`, wraps `*mut WT_CONNECTION`. Auto-closes on drop.
- **`Session<'conn>`** — created via `conn.open_session(config)`, wraps `*mut WT_SESSION`. Manages transactions.
- **`Cursor<'session>`** — created via `session.open_cursor(uri, config)`, wraps `*mut WT_CURSOR`. Used for all data access and mutation.

Supporting types:
- **`Item`** — wraps `WT_ITEM` for passing raw byte key/value data
- **`Modify`** — wraps `WT_MODIFY` for in-place value modifications
- **`TimestampType`** — enum for WiredTiger's transaction timestamp kinds
- **`PageLog` / `PageLogHandle`** — advanced page-level logging API

### FFI Layer (wt-sys/)

`wt-sys/build.rs` is the most complex file in the repo (~436 lines). It:
1. Reads `wt-sys/src/wiredtiger/dist/filelist` to determine which C source files to compile
2. Filters sources by OS (Darwin/Linux/Windows) and CPU architecture (x86, ARM64, RISC-V, etc.)
3. Generates `wiredtiger_config.h` with feature flags (compression codecs are off by default; encryption disabled)
4. Generates `wiredtiger.h` by substituting version tokens
5. Compiles the C library via the `cc` crate
6. Runs `bindgen` to auto-generate `wt-sys/src/lib.rs`

### Extensions (ext_crypto/)

WiredTiger extensions are `cdylib` crates that export `wiredtiger_extension_init()`. The `ext_crypto` crate shows this pattern with its own copy of WiredTiger bindings in `src/bindings.rs` (pre-generated, ~500KB).

## Key Implementation Notes

- All `unsafe` FFI calls are contained in `src/lib.rs`; the public API is entirely safe Rust
- WiredTiger's internal threading model means `Connection` is `Send + Sync`; `Session` and `Cursor` are `Send` but not `Sync`
- Drop implementations call the WiredTiger `close()` methods — never call close manually
- The `wt-sys` crate links a static WiredTiger library; the `ext_crypto` crate generates its own bindings and links dynamically
