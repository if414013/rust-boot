# Installation

This page walks you through adding rust-boot to your project and making sure your toolchain meets the requirements.

## Prerequisites

rust-boot requires **Rust 1.88 or later** (the Minimum Supported Rust Version). The framework uses `std::sync::LazyLock` and other modern standard library features that landed in recent stable releases.

Check your current Rust version:

```bash
rustc --version
```

If you need to update:

```bash
rustup update stable
```

## Adding rust-boot to Your Project

Create a new project (or use an existing one):

```bash
cargo new my-api
cd my-api
```

Add rust-boot as a dependency in your `Cargo.toml`:

```toml
[dependencies]
rust-boot = "0.1.0"
```

That single dependency pulls in everything you need: the core plugin system, Axum integration, and all built-in plugins. The `rust-boot` crate is a facade that re-exports from the underlying crates (`rust-boot-core`, `rust-boot-axum`, `rust-boot-plugins`).

## Workspace Dependency Versions

Under the hood, rust-boot depends on these key libraries. You generally don't need to add them yourself — they come through the `rust-boot` dependency — but it is useful to know the versions if you need to use them directly:

| Dependency | Version | Purpose |
|---|---|---|
| `tokio` | 1.35 | Async runtime (full features) |
| `serde` | 1.0 | Serialization/deserialization |
| `serde_json` | 1.0 | JSON support |
| `axum` | 0.7 | Web framework |
| `tower` | 0.4 | Middleware layer |
| `tower-http` | 0.5 | HTTP-specific middleware (tracing, CORS) |
| `uuid` | 1.6 | UUID generation (v4, serde support) |
| `jsonwebtoken` | 9.2 | JWT encoding/decoding |
| `moka` | 0.12 | In-memory cache (async) |
| `redis` | 0.24 | Redis client (async, tokio) |
| `metrics` | 0.21 | Metrics facade |
| `metrics-exporter-prometheus` | 0.12 | Prometheus exporter |
| `sea-orm` | 0.12 | Database ORM (Postgres, SQLite, MySQL) |
| `clap` | 4.4 | CLI argument parsing |
| `tracing` | 0.1 | Structured logging |

## Feature Flags

rust-boot currently ships as a single, fully-featured crate. All built-in plugins are included by default. Future releases will introduce feature flags to allow you to opt out of plugins you don't need, reducing compile times and binary size.

## Verifying the Installation

Create a minimal `src/main.rs` to verify everything compiles:

```rust
use rust_boot::prelude::*;

#[tokio::main]
async fn main() -> RustBootResult<()> {
    println!("rust-boot is ready!");
    Ok(())
}
```

Build and run:

```bash
cargo run
```

If you see `rust-boot is ready!` in your terminal, you are good to go.

## What's Next

Head to the [Quick Start](./quick-start.md) guide to build a complete CRUD API with authentication, caching, and monitoring.
