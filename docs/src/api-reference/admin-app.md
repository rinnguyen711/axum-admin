# AdminApp

`AdminApp` is the top-level builder for an axum-admin application. It collects configuration, registered entities, and authentication, then produces an Axum `Router`.

```rust
use axum_admin::AdminApp;

let router = AdminApp::new()
    .title("My App")
    .prefix("/admin")
    .auth(Box::new(my_auth))
    .register(my_entity)
    .into_router()
    .await;
```

---

## Constructor

### `AdminApp::new() -> Self`

Creates an `AdminApp` with defaults:

| Field | Default |
|---|---|
| `title` | `"Admin"` |
| `icon` | `"fa-solid fa-bolt"` |
| `prefix` | `"/admin"` |
| `upload_limit` | `10485760` (10 MiB) |
| `auth` | None (must be set before `into_router`) |

---

## Builder Methods

### `title(self, title: &str) -> Self`

Sets the application name shown in the sidebar header and browser title.

```rust
.title("My Admin")
```

---

### `icon(self, icon: &str) -> Self`

Sets the Font Awesome icon class for the app logo in the sidebar. Defaults to `"fa-solid fa-bolt"`.

```rust
.icon("fa-solid fa-database")
```

---

### `prefix(self, prefix: &str) -> Self`

Sets the URL prefix for all admin routes. Defaults to `"/admin"`.

```rust
.prefix("/staff")
```

---

### `register(self, entry: impl Into<AdminEntry>) -> Self`

Registers an `EntityAdmin` or `EntityGroupAdmin` with the app. Can be called multiple times.

```rust
.register(EntityAdmin::new::<()>("posts"))
.register(my_entity_group)
```

---

### `auth(self, auth: Box<dyn AdminAuth>) -> Self`

Sets the authentication provider. Required before calling `into_router()`. Panics at router construction if not set.

```rust
.auth(Box::new(DefaultAdminAuth::new("admin", "secret")))
```

---

### `seaorm_auth(self, auth: SeaOrmAdminAuth) -> Self`

*(Requires `seaorm` feature)*

Configures SeaORM-backed authentication with full RBAC support. Automatically sets the enforcer and auth provider. This is the recommended auth method when using SeaORM.

```rust
.seaorm_auth(SeaOrmAdminAuth::new(db.clone()).await?)
```

---

### `upload_limit(self, bytes: usize) -> Self`

Sets the maximum multipart upload body size in bytes. Defaults to `10 * 1024 * 1024` (10 MiB).

```rust
.upload_limit(50 * 1024 * 1024) // 50 MiB
```

---

### `template(self, name: &str, content: &str) -> Self`

Overrides a built-in template or adds a new one by name. Template names must match the filenames used by the renderer (e.g. `"home.html"`, `"layout.html"`, `"form.html"`). Inline templates set via this method take precedence over `template_dir` templates.

```rust
.template("home.html", include_str!("templates/home.html"))
```

---

### `template_dir(self, path: impl Into<PathBuf>) -> Self`

Loads templates from a directory on disk at startup. Any `.html` file whose name matches a built-in template overrides it; unknown names are added as new templates. Multiple directories can be registered; later calls take precedence over earlier ones, and `.template()` always wins over `.template_dir()`.

```rust
.template_dir("templates/admin")
```

---

## Finalizer

### `async fn into_router(self) -> Router`

Consumes the `AdminApp` and returns a configured Axum `Router` with all admin routes, static assets, middleware, and authentication layers applied.

**Panics** if `.auth()` (or `.seaorm_auth()`) was not called before this method.

```rust
let router = AdminApp::new()
    .title("My App")
    .auth(Box::new(my_auth))
    .register(users_entity)
    .into_router()
    .await;

let app = Router::new().merge(router);
```
