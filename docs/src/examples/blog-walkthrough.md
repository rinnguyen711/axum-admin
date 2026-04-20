# Blog Example Walkthrough

The blog example is the canonical axum-admin demo. It registers three related entities — **Categories**, **Posts**, and **Tags** — and shows foreign keys, many-to-many relationships, enum fields, search, and filters all working together.

Source: `examples/blog/`

---

## What it builds

| Entity | Table | Notable fields |
|--------|-------|---------------|
| Category | `categories` | `id`, `name` |
| Post | `posts` | `id`, `title`, `body`, `status` (enum), `category_id` (FK) |
| Tag | `tags` | `id`, `name` |
| Post–Tag join | `post_tags` | `post_id`, `tag_id` |

The admin UI exposes:

- Full CRUD for all three entities
- Post list with search across `title` and `body`
- Post list with filters on `status` and `category_id`
- A foreign-key picker for category on the post form
- A many-to-many tag selector on the post form
- Auto-detected enum select for `status` (Draft / Published)
- A default admin user (`admin` / `admin`) created on first boot

---

## Database setup

The example ships a `docker-compose.yml` that starts a Postgres 16 container:

```yaml
services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: blog
      POSTGRES_PASSWORD: blog
      POSTGRES_DB: blog
    ports:
      - "5432:5432"
```

Start the database:

```bash
cd examples/blog
docker compose up -d db
```

The app defaults to `postgres://blog:blog@localhost:5432/blog`. Override with `DATABASE_URL` if needed.

---

## Running the example

```bash
cd examples/blog
cargo run
```

Migrations run automatically on startup. Open `http://localhost:3000/admin` and log in with `admin` / `admin`.

To run the full stack (database + app) in containers:

```bash
docker compose up
```

---

## Walkthrough: `main.rs`

### Entity models

All three entities are defined inline in `main.rs` as Sea-ORM entities. This keeps the example self-contained — in a real project these would live in their own modules or crates.

```rust
mod post {
    #[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum)]
    #[sea_orm(rs_type = "String", db_type = "String(StringLen::None)")]
    pub enum Status {
        #[sea_orm(string_value = "draft")]
        Draft,
        #[sea_orm(string_value = "published")]
        Published,
    }

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub title: String,
        pub body: String,
        pub status: Status,
        pub category_id: Option<i32>,
    }
    // ...
}
```

Key points:
- `Status` derives `DeriveActiveEnum` — axum-admin detects this automatically and renders a `<select>` with "Draft" and "Published" options. No extra configuration needed.
- `category_id` is `Option<i32>` — nullable, meaning a post does not have to belong to a category.

### Startup sequence

```rust
#[tokio::main]
async fn main() {
    let db = connect_db().await;
    migration::Migrator::up(&db, None).await.expect("...");
    let router = admin::build(db).await;
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, router).await.unwrap();
}
```

1. **Connect** — reads `DATABASE_URL` from the environment, falls back to the Docker default.
2. **Migrate** — runs all pending Sea-ORM migrations before accepting traffic. This is safe to run on every boot.
3. **Build** — delegates to `admin::build(db)`, which returns an Axum `Router`.
4. **Serve** — hands the router to Axum's standard server loop.

---

## Walkthrough: `admin.rs`

### Auth setup

```rust
let auth = SeaOrmAdminAuth::new(db.clone()).await?;
auth.ensure_user("admin", "admin").await?;
```

`SeaOrmAdminAuth` creates the `admin_users` and `admin_roles` tables (via its own migrations) and wires up session-based login. `ensure_user` is idempotent — it creates the user on first run, does nothing on subsequent boots.

### AdminApp configuration

```rust
AdminApp::new()
    .title("Blog Admin")
    .icon("fa-solid fa-newspaper")
    .prefix("/admin")
    .template_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/templates"))
    .seaorm_auth(auth)
```

- `.title()` — sets the name shown in the nav header.
- `.icon()` — Font Awesome class for the logo icon.
- `.prefix("/admin")` — all admin routes are mounted under this path.
- `.template_dir()` — path to local Tera templates that override defaults. The `concat!(env!(...))` macro resolves the absolute path at compile time, which is required when the binary is run from a different working directory.
- `.seaorm_auth(auth)` — attaches the authentication layer.

### Entity group

```rust
.register(
    EntityGroupAdmin::new("Blog")
        .register(categories_admin)
        .register(posts_admin)
        .register(tags_admin)
)
```

`EntityGroupAdmin` is a named section in the sidebar. All three entities appear under a "Blog" heading. You can have multiple groups for larger applications.

### Categories entity

```rust
EntityAdmin::from_entity::<category::Entity>("categories")
    .label("Categories")
    .icon("fa-solid fa-folder")
    .list_display(vec!["id".to_string(), "name".to_string()])
    .search_fields(vec!["name".to_string()])
    .adapter(Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone())))
```

- `from_entity` — infers all fields from the Sea-ORM entity. Without explicit `.field()` calls, the admin generates default inputs for each column.
- `list_display` — controls which columns appear in the list view and their order.
- `search_fields` — enables the search box and specifies which columns to query with a `LIKE` filter.
- `adapter` — the Sea-ORM adapter handles all database reads and writes for this entity.

### Tags entity

Tags is identical to categories in structure — a simple two-column entity with search on `name`. It follows the same minimal pattern.

### Posts entity — the interesting one

```rust
EntityAdmin::from_entity::<post::Entity>("posts")
    .label("Posts")
    .icon("fa-solid fa-file-lines")
    .field(
        Field::text("title")
            .required()
            .min_length(3)
            .max_length(255),
    )
    .field(Field::textarea("body").max_length(10000))
    .field(Field::foreign_key(
        "category_id",
        "Category",
        Box::new(SeaOrmAdapter::<category::Entity>::new(db.clone())),
        "id",
        "name",
    ))
    .field(
        Field::many_to_many(
            "tags",
            Box::new(SeaOrmManyToManyAdapter::new(
                db.clone(),
                "post_tags",   // join table
                "post_id",     // FK to this entity
                "tag_id",      // FK to related entity
                "tags",        // related table
                "id",          // related PK
                "name",        // related display column
            )),
        )
        .label("Tags"),
    )
    .search_fields(vec!["title".to_string(), "body".to_string()])
    .filter_fields(vec!["status".to_string(), "category_id".to_string()])
    .adapter(Box::new(SeaOrmAdapter::<post::Entity>::new(db.clone())))
```

**Field overrides** — explicit `.field()` calls replace the auto-generated defaults for those columns:

- `Field::text("title")` — single-line text input with server-side validation. `.required()` makes the field mandatory. `.min_length(3).max_length(255)` adds length constraints enforced on save.
- `Field::textarea("body")` — multi-line textarea. `.max_length(10000)` prevents oversized payloads.

**Foreign key field** — `Field::foreign_key` renders a dropdown populated by querying the categories adapter. The four string arguments are: column name on `posts`, display label, adapter, PK column name, display column name. When editing a post, the dropdown shows category names but stores category IDs.

**Many-to-many field** — `Field::many_to_many` with `SeaOrmManyToManyAdapter` handles the `post_tags` join table transparently. On save, axum-admin diffs the selected tag IDs against the current join table rows and issues the appropriate inserts and deletes. The adapter arguments map directly to the join table schema.

**Search and filters** — `search_fields` queries both `title` and `body` columns. `filter_fields` adds sidebar filter widgets for `status` (rendered as an enum select because the column maps to `Status`) and `category_id` (rendered as a foreign-key dropdown).

---

## Adapting this to your own project

1. Define your Sea-ORM entities (or use existing ones from your domain crate).
2. Call `AdminApp::new()` and configure title, prefix, and auth.
3. For each entity, create an `EntityAdmin::from_entity` and attach a `SeaOrmAdapter`.
4. Add explicit `Field` definitions only where you need validation, custom widgets, or relationship pickers — everything else is auto-detected.
5. Group related entities with `EntityGroupAdmin`.
6. Call `.into_router().await` and merge the resulting `Router` into your Axum application.
