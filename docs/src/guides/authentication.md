# Authentication

axum-admin ships two auth backends out of the box and lets you plug in your own via the `AdminAuth` trait.

## The AdminAuth Trait

```rust
#[async_trait]
pub trait AdminAuth: Send + Sync {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AdminUser, AdminError>;

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError>;
}
```

The framework calls `authenticate` on login and `get_session` on every protected request. Both return an `AdminUser`:

```rust
pub struct AdminUser {
    pub username: String,
    pub session_id: String,
    /// true = bypasses all permission checks (superuser access)
    pub is_superuser: bool,
}
```

## DefaultAdminAuth

`DefaultAdminAuth` is an in-memory backend. Credentials are configured at startup; sessions live for the process lifetime. Passwords are hashed with bcrypt.

```rust
use axum_admin::auth::DefaultAdminAuth;

let auth = DefaultAdminAuth::new()
    .add_user("admin", "s3cret");

AdminApp::new()
    .auth(Box::new(auth))
    // ...
```

`add_user` is builder-style and can be chained for multiple users. Every user created through `DefaultAdminAuth` is implicitly a superuser — it bypasses all permission checks.

This backend is suitable for local development and single-user deployments. It has no persistence: users and sessions are lost on restart.

## SeaOrmAdminAuth

`SeaOrmAdminAuth` stores users and sessions in PostgreSQL and integrates Casbin for RBAC. It requires the `seaorm` feature flag.

### Setup

```rust
use axum_admin::adapters::seaorm_auth::SeaOrmAdminAuth;

let auth = SeaOrmAdminAuth::new(db.clone()).await?;

// Seed a default user only when the users table is empty.
auth.ensure_user("admin", "change-me").await?;

AdminApp::new()
    .seaorm_auth(auth)
    // ...
```

`SeaOrmAdminAuth::new` runs database migrations automatically (idempotent). It creates the `auth_users`, `auth_sessions`, and Casbin policy tables.

### ensure_user

```rust
pub async fn ensure_user(&self, username: &str, password: &str) -> Result<(), AdminError>
```

Creates a user with `is_superuser = true` and assigns the `admin` role **only if no users exist yet**. Safe to call on every application startup.

### create_user

```rust
pub async fn create_user(
    &self,
    username: &str,
    password: &str,
    is_superuser: bool,
) -> Result<(), AdminError>
```

Creates a user unconditionally. Passwords are hashed with Argon2.

### change_password

```rust
pub async fn change_password(
    &self,
    username: &str,
    old_password: &str,
    new_password: &str,
) -> Result<(), AdminError>
```

Verifies the old password before storing the new hash.

### Wiring with seaorm_auth

`AdminApp::seaorm_auth` is a convenience method that sets both the auth backend and the Casbin enforcer in one call:

```rust
pub fn seaorm_auth(mut self, auth: SeaOrmAdminAuth) -> Self
```

When `.seaorm_auth()` is used, the Users and Roles management pages appear automatically in the sidebar navigation.

## Custom Auth Backend

Implement `AdminAuth` on any struct and pass it with `.auth()`:

```rust
use axum_admin::auth::{AdminAuth, AdminUser};
use axum_admin::error::AdminError;
use async_trait::async_trait;

pub struct MyAuth;

#[async_trait]
impl AdminAuth for MyAuth {
    async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AdminUser, AdminError> {
        // verify credentials, create a session record, return AdminUser
        todo!()
    }

    async fn get_session(&self, session_id: &str) -> Result<Option<AdminUser>, AdminError> {
        // look up session by ID, return None if expired or missing
        todo!()
    }
}

AdminApp::new()
    .auth(Box::new(MyAuth))
```

Custom backends always use the basic `.auth()` path. The Casbin enforcer and the Users/Roles nav pages are only available through `.seaorm_auth()`.
