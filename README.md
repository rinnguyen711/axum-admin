# axum-admin

A modern admin dashboard framework for [Axum](https://github.com/tokio-rs/axum). Register your entities and get a full CRUD dashboard — search, filtering, pagination, bulk actions, custom actions, and built-in authentication — with zero frontend build step.

Inspired by Django Admin and Laravel Nova.

## Features

- CRUD out of the box — list, create, edit, delete for any entity
- Server-side rendering via MiniJinja, no JS framework required
- HTMX + Alpine.js embedded, no CDN or build step
- Session-based auth with bcrypt; swap in your own backend
- Sidebar groups with collapsible sections and custom icons
- Filters, search, column sorting, pagination
- Bulk actions (delete, CSV export) and per-record custom actions
- Lifecycle hooks: `before_save`, `after_delete`
- Template override support
- ORM-agnostic via `DataAdapter` trait
- First-party SeaORM adapter behind the `seaorm` feature flag

## Quick start

```toml
[dependencies]
axum-admin = { version = "0.1", features = ["seaorm"] }
```

See [examples/blog](examples/blog) for a full working example with SeaORM and PostgreSQL.

Full documentation coming soon.

## License

MIT
