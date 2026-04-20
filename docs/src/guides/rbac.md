# RBAC

axum-admin uses [Casbin](https://casbin.org/) for role-based access control. RBAC is only available when the `seaorm` feature is enabled and the app is configured with `.seaorm_auth()`.

## How It Works

The permission model follows the pattern `(subject, object, action)`:

- **subject** — a role name stored internally as `role:<name>` (e.g. `role:admin`)
- **object** — an entity name (e.g. `posts`)
- **action** — one of `view`, `create`, `edit`, `delete`

Superusers (`is_superuser = true`) bypass all Casbin checks. For regular users, every request checks that the user's assigned role has the required `(entity, action)` pair.

## Built-in Roles

`seed_roles` pre-populates two roles for every registered entity:

- **admin** — `view`, `create`, `edit`, `delete` on all entities
- **viewer** — `view` only on all entities

```rust
pub async fn seed_roles(&self, entity_names: &[String]) -> Result<(), AdminError>
```

Call this after all entities are registered. The method is idempotent — it skips rules that already exist. The `AdminApp::seaorm_auth` builder calls `seed_roles` automatically when building the router.

## Assigning Roles

```rust
pub async fn assign_role(&self, username: &str, role: &str) -> Result<(), AdminError>
```

Assigns a role to a user. A user has exactly one role at a time — any previous role is removed first.

```rust
auth.assign_role("alice", "viewer").await?;
auth.assign_role("bob", "admin").await?;
```

## Role Management API

### create_role

```rust
pub async fn create_role(
    &self,
    name: &str,
    permissions: &[(String, String)],
) -> Result<(), AdminError>
```

Creates a role with a specific set of `(entity, action)` pairs. Returns `AdminError::Conflict` if the role already exists.

```rust
auth.create_role("editor", &[
    ("posts".to_string(), "view".to_string()),
    ("posts".to_string(), "edit".to_string()),
]).await?;
```

### get_role_permissions

```rust
pub fn get_role_permissions(&self, name: &str) -> Vec<(String, String)>
```

Returns all `(entity, action)` pairs assigned to the role.

### update_role_permissions

```rust
pub async fn update_role_permissions(
    &self,
    name: &str,
    permissions: &[(String, String)],
) -> Result<(), AdminError>
```

Replaces the full permission set for the role.

### delete_role

```rust
pub async fn delete_role(&self, name: &str) -> Result<(), AdminError>
```

Deletes the role and all its policies. Returns `AdminError::Conflict` if any users are currently assigned to it.

### list_roles

```rust
pub fn list_roles(&self) -> Vec<String>
```

Returns all role names currently in the policy store (without the `role:` prefix).

### get_user_role

```rust
pub fn get_user_role(&self, username: &str) -> Option<String>
```

Returns the role currently assigned to the user, or `None` if unassigned.

## Entity-Level Permission Guards

Define which permission is required for each operation on an entity:

```rust
EntityAdmin::from_entity::<posts::Entity>("posts")
    .require_view("posts.view")
    .require_create("posts.create")
    .require_edit("posts.edit")
    .require_delete("posts.delete")
```

Or use the shortcut to guard all four actions with the same permission string:

```rust
.require_role("posts.admin")
```

When the permission string matches the `entity.action` pattern (e.g. `"posts.view"`), Casbin checks `(username, posts, view)`. If no permission string is set, the framework auto-derives `entity_name.action` when an enforcer is present.

## Accessing the Enforcer

The Casbin enforcer is exposed for advanced use:

```rust
pub fn enforcer(&self) -> Arc<tokio::sync::RwLock<casbin::Enforcer>>
```

```rust
let enforcer = auth.enforcer();
let guard = enforcer.read().await;
// use casbin CoreApi / MgmtApi / RbacApi directly
```

## Startup Recipe

```rust
let auth = SeaOrmAdminAuth::new(db.clone()).await?;
auth.ensure_user("admin", "change-me").await?;

let app = AdminApp::new()
    .title("My App")
    .seaorm_auth(auth)
    .register(
        EntityAdmin::from_entity::<posts::Entity>("posts")
            .require_view("posts.view")
            .require_edit("posts.edit")
    )
    .into_router();
```

`seed_roles` is called automatically inside `.into_router()`, so the `admin` and `viewer` roles are available immediately after startup.
