# Lifecycle Hooks

Lifecycle hooks let you run synchronous logic at key points in a record's lifecycle — before a save completes or after a delete. They are registered per entity.

## before_save

Called before a record is written to the database. Receives a mutable reference to the form data so you can validate, transform, or reject the save.

### Signature

```rust
pub fn before_save<F>(mut self, f: F) -> Self
where
    F: Fn(&mut HashMap<String, Value>) -> Result<(), AdminError> + Send + Sync + 'static,
```

The `HashMap<String, Value>` contains the submitted field values keyed by field name. Return `Ok(())` to allow the save to proceed, or return `Err(AdminError::...)` to abort it.

### Examples

**Normalize a field before saving:**

```rust
use std::collections::HashMap;
use serde_json::Value;
use axum_admin::error::AdminError;

EntityAdmin::from_entity::<posts::Entity>("posts")
    .before_save(|data: &mut HashMap<String, Value>| {
        if let Some(Value::String(title)) = data.get_mut("title") {
            *title = title.trim().to_string();
        }
        Ok(())
    })
```

**Validate required business logic:**

```rust
.before_save(|data: &mut HashMap<String, Value>| {
    let slug = data.get("slug").and_then(|v| v.as_str()).unwrap_or("");
    if slug.is_empty() {
        return Err(AdminError::Validation("Slug cannot be empty".to_string()));
    }
    Ok(())
})
```

**Inject a computed field:**

```rust
.before_save(|data: &mut HashMap<String, Value>| {
    let now = chrono::Utc::now().to_rfc3339();
    data.insert("updated_at".to_string(), Value::String(now));
    Ok(())
})
```

## after_delete

Called after a record is deleted from the database. Receives the primary key value of the deleted record.

### Signature

```rust
pub fn after_delete<F>(mut self, f: F) -> Self
where
    F: Fn(&Value) -> Result<(), AdminError> + Send + Sync + 'static,
```

The `&Value` is the primary key of the deleted record. Returning `Err` from `after_delete` does not un-delete the record — the deletion has already occurred.

### Examples

**Log deletions:**

```rust
EntityAdmin::from_entity::<posts::Entity>("posts")
    .after_delete(|id: &Value| {
        tracing::info!("Post deleted: {}", id);
        Ok(())
    })
```

**Clean up related data:**

```rust
.after_delete(|id: &Value| {
    // e.g. remove associated files, notify a webhook, etc.
    let id_str = id.to_string();
    // synchronous cleanup...
    Ok(())
})
```

## Notes

- Both hooks are **synchronous**. For async work (database calls, HTTP requests), use `std::thread::spawn` or a channel to offload to an async runtime, or restructure the logic to happen before/after the admin handler is called.
- Only one `before_save` and one `after_delete` hook can be registered per entity. Calling either method a second time replaces the previous hook.
- Hook functions must be `Send + Sync + 'static`, so they cannot capture non-`Send` types or references to short-lived data.
