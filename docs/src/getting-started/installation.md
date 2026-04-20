# Installation

Add `axum-admin` to your `Cargo.toml`:

```sh
cargo add axum-admin --features seaorm
cargo add axum tokio --features tokio/full
```

Or add manually:

```toml
[dependencies]
axum-admin = { version = "0.1", features = ["seaorm"] }
axum = { version = "0.7", features = ["macros"] }
tokio = { version = "1", features = ["full"] }
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `seaorm` | Enables SeaORM adapter, RBAC via Casbin, and `SeaOrmAdminAuth` |

Without the `seaorm` feature, you must provide your own `DataAdapter` and `AdminAuth` implementations.

## Requirements

- Rust 1.75+
- PostgreSQL (when using the `seaorm` feature)
