# Blog Example

The `examples/blog/` directory contains a complete working example using PostgreSQL and SeaORM.

## Prerequisites

- Docker (for the database)
- Rust 1.75+

## Running the example

```sh
cd examples/blog
docker compose up -d        # start PostgreSQL
cargo run                   # run migrations + start server
```

Visit `http://localhost:3000/admin` and log in with `admin` / `secret`.

## What's included

- `Post` entity with title, body, and published flag
- `Category` entity with a many-to-many relationship to posts
- SeaORM migrations for the blog tables
- Full CRUD for both entities
- RBAC: an `editor` role with read-only access to posts

See `examples/blog/src/admin.rs` for the full registration code.
