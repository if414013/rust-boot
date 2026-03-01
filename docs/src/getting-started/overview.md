# Getting Started — Overview

This section introduces the philosophy behind rust-boot, explains why it exists, and gives you a mental model for how the framework fits together before you write any code.

## Philosophy

rust-boot is built on a simple premise: Rust developers deserve the same rapid-development experience that Spring Boot provides to the Java ecosystem, without sacrificing the safety, performance, and correctness guarantees that Rust is known for.

The framework follows these guiding principles:

**Batteries included, but removable.** Every major capability — authentication, caching, monitoring, event sourcing — ships as a built-in plugin. You opt in to what you need by registering plugins at startup. If you don't register a plugin, it doesn't exist in your application. There is no hidden overhead.

**Convention over configuration.** Sensible defaults let you get a working API running in minutes. Configuration is layered: defaults are overridden by config files, which are overridden by environment variables. You only configure what you need to change.

**Plugin architecture at the core.** The plugin system is not an afterthought — it is the foundation. Every built-in capability is implemented as a plugin using the same `CrudPlugin` trait that you use to write your own. This means the framework eats its own dog food, and you can extend or replace any part of it.

**Rust safety, not Rust ceremony.** The framework handles the boilerplate (routing, serialization, error mapping, pagination) so you can focus on your domain logic. Procedural macros generate the repetitive code. The prelude module gives you a single import for the most common types.

## Inspiration from Spring Boot

If you have used Spring Boot, many concepts will feel familiar:

| Spring Boot | rust-boot |
|---|---|
| `@SpringBootApplication` | `PluginRegistry` + `init_all()` |
| `@RestController` | `CrudRouterBuilder` |
| `@Service` | `CrudService` trait |
| `@Repository` | `CrudRepository` trait |
| Spring Security | `AuthPlugin` with JWT + RBAC |
| Spring Cache | `CachingPlugin` with Moka/Redis |
| Spring Actuator | `MonitoringPlugin` with Prometheus |
| Spring Events | `EventSourcingPlugin` |

The key difference is that rust-boot is explicit rather than magical. There are no annotation processors scanning your classpath at runtime. Plugins are registered in code, dependencies are resolved at startup, and the type system catches misconfigurations at compile time.

## How the Pieces Fit Together

A typical rust-boot application follows this flow:

1. **Configure plugins** — Create configuration structs for the plugins you need (cache TTL, JWT secret, metrics endpoint, etc.).
2. **Register plugins** — Add each plugin to a `PluginRegistry`. The registry handles dependency ordering automatically.
3. **Initialize** — Call `registry.init_all().await` to initialize all plugins in the correct order.
4. **Build routers** — Use `CrudRouterBuilder` to define your API endpoints. Map HTTP verbs to handler functions.
5. **Start serving** — Pass the router to Axum's `serve()` and you are live.

This top-down flow means you always know exactly what your application is doing at startup. There is no classpath scanning, no auto-configuration magic, and no hidden beans. Everything is explicit, typed, and visible in your `main()` function.

## What's Next

- [Installation](./installation.md) — Add rust-boot to your project and verify your toolchain.
- [Quick Start](./quick-start.md) — Build a complete CRUD API from scratch in a single file.
