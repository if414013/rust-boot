# Guides

This section contains hands-on tutorials that walk you through building real applications with rust-boot. Each guide builds on the concepts from the [Core](../core/overview.md) and [Plugins](../plugins/overview.md) documentation, showing how the pieces fit together in practice.

## Available Guides

### [Basic API Tutorial](./basic-api-tutorial.md)

Build a complete CRUD REST API from scratch. Covers defining models with the `CrudModel` macro, setting up repositories and services, configuring Axum routes, and handling errors. This is the best starting point if you're new to rust-boot.

### [Custom Plugin Tutorial](./custom-plugin-tutorial.md)

Step-by-step guide to building your own plugin. Walks through implementing the `CrudPlugin` trait, managing shared state with `PluginContext`, declaring dependencies, and testing your plugin through the full lifecycle.

### [Database Setup](./database-setup.md)

Configure rust-boot to work with different databases. Covers connection pooling, migrations, and integrating the repository layer with PostgreSQL, SQLite, and other backends.

## Recommended Reading Order

If you're just getting started with rust-boot, we recommend this path:

1. [Getting Started](../getting-started/overview.md) — Install and run your first app
2. [Architecture Overview](../architecture/overview.md) — Understand the crate structure and design
3. [Basic API Tutorial](./basic-api-tutorial.md) — Build your first API
4. [Plugin Overview](../plugins/overview.md) — Learn the plugin system
5. [Custom Plugin Tutorial](./custom-plugin-tutorial.md) — Extend the framework

## Prerequisites

All guides assume you have:

- Rust 1.88 or later (the framework's MSRV)
- Basic familiarity with async Rust and Tokio
- `cargo` installed and working

Some guides may have additional requirements (e.g., a running PostgreSQL instance for the database guide), which are noted at the top of each guide.
