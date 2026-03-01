# Reference

This section provides detailed reference documentation for every major component in the rust-boot framework. Use these pages when you need precise information about types, attributes, commands, or APIs.

## What's in This Section

### [CrudModel Macro](./crud-model-macro.md)

The `#[derive(CrudModel)]` procedural macro auto-generates SeaORM entities, DTOs, and OpenAPI schemas from a single annotated struct. The reference covers:

- All struct-level attributes (`table_name`, `soft_delete`, `timestamps`)
- All field-level attributes (`primary_key`, `column_name`, `nullable`, `skip_dto`, `validation`)
- Validation rules (`email`, `url`, `min_length`, `max_length`, `pattern`, custom)
- What code gets generated (entity module, CreateDTO, UpdateDTO, ResponseDTO)

### [CLI Reference](./cli.md)

The `rust-boot` CLI scaffolds new projects and generates model boilerplate. The reference covers:

- `rust-boot new <name>` — create a new project from a template
- `rust-boot generate model <name>` — scaffold a CrudModel struct
- Available flags and options

### [API Reference](./api-reference.md)

A comprehensive catalog of every public type in the framework, organized by crate:

- **rust-boot-core** — Configuration, plugin system, repository traits, service layer, error types
- **rust-boot-axum** — Router builder, response helpers, pagination types
- **rust-boot-plugins** — Caching, authentication, monitoring, event sourcing
- **rust-boot-macros** — The CrudModel derive macro

## How to Use This Section

If you're building an application, start with the [Guides](../guides/basic-api-tutorial.md) for step-by-step tutorials. Come back here when you need to look up a specific type signature, attribute option, or CLI flag.

If you're writing a custom plugin, the [API Reference](./api-reference.md) section on the plugin system will be your primary resource, alongside the [Custom Plugin Tutorial](../guides/custom-plugin-tutorial.md).
