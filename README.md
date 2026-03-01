# rust-boot

[![Crates.io](https://img.shields.io/crates/v/rust-boot.svg)](https://crates.io/crates/rust-boot)
[![Docs.rs](https://docs.rs/rust-boot/badge.svg)](https://docs.rs/rust-boot)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/yourusername/rust-boot/workflows/CI/badge.svg)](https://github.com/yourusername/rust-boot/actions)

A batteries-included CRUD API framework for Rust, inspired by Spring Boot.

## Features

- **Plugin System**: Extensible architecture with lifecycle hooks (init, startup, shutdown).
- **JWT Authentication**: Built-in support for JWT tokens with Role-Based Access Control (RBAC).
- **Caching**: Flexible caching abstraction with Moka (in-memory) and Redis backends.
- **Monitoring**: Integrated Prometheus metrics and customizable health checks.
- **Event Sourcing**: Domain event system with pluggable event stores.
- **CLI Scaffolding**: Command-line tool for rapid project generation.

## Quick Start

```rust
use rust_boot::prelude::*;

#[tokio::main]
async fn main() -> RustBootResult<()> {
    // 1. Setup plugins
    let mut registry = PluginRegistry::new();
    
    registry.register(CachingPlugin::new(CacheConfig::default()))?;
    registry.register(MonitoringPlugin::new(MetricsConfig::default()))?;
    registry.register(AuthPlugin::new(JwtConfig::new("your-secret")))?;
    
    registry.init_all().await?;

    // 2. Define your API
    let state = AppState { /* ... */ };
    let app = CrudRouterBuilder::<AppState>::new(CrudRouterConfig::new("/api/users"))
        .list(list_users)
        .get(get_user)
        .create(create_user)
        .build()
        .with_state(state);

    // 3. Start server
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
```

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
rust-boot = "0.1.0"
```

## Architecture

The framework consists of several specialized crates:

- **rust-boot**: The main facade crate providing high-level re-exports and the primary API.
- **rust-boot-core**: The foundation containing the plugin system, configuration management, and error handling.
- **rust-boot-axum**: Deep integration with the Axum web framework, providing CRUD routers and handlers.
- **rust-boot-plugins**: A collection of ready-to-use plugins for authentication, caching, monitoring, and events.
- **rust-boot-macros**: Procedural macros for code generation (DTOs, entities, and CRUD implementations).
- **rust-boot-cli**: A command-line interface for scaffolding new projects and generating code.

## Examples

Check out the [examples/](rust-boot/examples/) directory for complete usage demonstrations:

- [Basic API](rust-boot/examples/basic_api.rs): A full CRUD API with authentication and caching.
- [Custom Plugin](rust-boot/examples/custom_plugin.rs): How to extend the framework with your own plugins.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
