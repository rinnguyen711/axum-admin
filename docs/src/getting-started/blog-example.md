# Blog Example

The `examples/blog/` directory contains a complete working example using PostgreSQL and SeaORM.

## Prerequisites

- Docker (for the database)
- Rust 1.75+

## Running the example

```sh
cd examples/blog
docker compose up -d db     # start PostgreSQL
cargo run                   # run migrations + start server
```

Visit `http://localhost:3000/admin` and log in with `admin` / `admin`.

## What's included

- `Category` entity with id and name fields, searchable by name
- `Post` entity with title, body, status, a foreign-key to Category, and a many-to-many relationship to Tags; searchable by title and body, filterable by status and category
- `Tag` entity with id and name fields, searchable by name
- All three entities grouped under a "Blog" sidebar section
- SeaORM migrations for the blog tables
- Full CRUD for all entities

See `examples/blog/src/admin.rs` for the full registration code.
