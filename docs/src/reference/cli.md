# CLI Reference

The `rust-boot` CLI is a scaffolding tool that generates new projects and model boilerplate. It uses [Tera](https://keats.github.io/tera/) templates internally and is built with [clap](https://docs.rs/clap/).

## Installation

```bash
cargo install rust-boot-cli
```

## Global Options

| Flag | Description |
|------|-------------|
| `-v, --verbose` | Enable verbose output |
| `--version` | Print version information |
| `-h, --help` | Print help |

---

## `rust-boot new`

Create a new rust-boot project from a template.

```bash
rust-boot new <NAME> [OPTIONS]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<NAME>` | Name of the project to create (used as directory name and in Cargo.toml) |

### Options

| Flag | Default | Description |
|------|---------|-------------|
| `-t, --template <TEMPLATE>` | `basic` | Template to use for project generation |

### What Gets Generated

The `new` command renders four files using Tera templates:

| File | Description |
|------|-------------|
| `Cargo.toml` | Project manifest with rust-boot dependency |
| `src/main.rs` | Application entry point with basic setup |
| `src/lib.rs` | Library root |
| `.gitignore` | Standard Rust gitignore |

Template variables available:

| Variable | Value |
|----------|-------|
| `project_name` | The name you provided (e.g., `my-app`) |
| `project_name_snake` | Snake-case version (e.g., `my_app`) |
| `rust_boot_version` | Current rust-boot version (e.g., `0.1.0`) |

### Examples

```bash
# Create a new project with the default template
rust-boot new my-api

# Create with a specific template
rust-boot new my-api --template basic
```

### Output

```
Generated project: my-api

Generated files:
  - Cargo.toml
  - src/main.rs
  - src/lib.rs
  - .gitignore
```

---

## `rust-boot generate`

Generate a model, handler, or other artifact. Aliased as `rust-boot g`.

```bash
rust-boot generate <TYPE> <NAME>
rust-boot g <TYPE> <NAME>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<TYPE>` | Type of artifact to generate |
| `<NAME>` | Name of the artifact (e.g., `User`, `Product`) |

### Supported Artifact Types

| Type | Aliases | Description |
|------|---------|-------------|
| `model` | `entity` | Generate a CrudModel struct with derive macro |

### Name Handling

The CLI automatically converts names between cases:

| Input | PascalCase (struct) | snake_case (file) |
|-------|--------------------|--------------------|
| `user` | `User` | `user.rs` |
| `user_profile` | `UserProfile` | `user_profile.rs` |
| `user-profile` | `UserProfile` | `user_profile.rs` |
| `UserProfile` | `UserProfile` | `user_profile.rs` |

### Examples

```bash
# Generate a User model
rust-boot generate model User

# Same thing with the alias
rust-boot g model User

# Using "entity" as an alias for "model"
rust-boot generate entity Product
```

### Output

The generated model file contains a struct with `#[derive(CrudModel)]` and standard field annotations:

```rust
use rust_boot_macros::CrudModel;

#[derive(CrudModel)]
#[crud_model(table_name = "users")]
pub struct User {
    #[crud_field(primary_key)]
    pub id: i64,

    pub name: String,
}
```

```
Generated user.rs: User
```

---

## Template Engine

The CLI uses Tera for template rendering. Templates are compiled into the binary at build time — no external template files are needed at runtime.

Available templates:

| Template ID | Used By | Renders |
|-------------|---------|---------|
| `cargo_toml` | `new` | `Cargo.toml` |
| `main_rs` | `new` | `src/main.rs` |
| `lib_rs` | `new` | `src/lib.rs` |
| `gitignore` | `new` | `.gitignore` |
| `model` | `generate model` | Model struct file |

---

## See Also

- [CrudModel Macro Reference](./crud-model-macro.md) — Details on the generated `#[derive(CrudModel)]` attributes
- [Basic API Tutorial](../guides/basic-api-tutorial.md) — Building an API from a generated project
