# Custom Actions

Custom actions add buttons to the list view or detail form that trigger arbitrary async logic — sending emails, calling external APIs, bulk-processing records, etc.

## Core Types

### ActionTarget

Where the action button is rendered:

```rust
pub enum ActionTarget {
    List,    // Bulk action toolbar on the list page (operates on selected rows)
    Detail,  // Action button on the edit/detail form (operates on a single record)
}
```

### ActionContext

Passed to the handler at invocation time:

```rust
pub struct ActionContext {
    pub ids: Vec<Value>,                    // Selected record IDs
    pub params: HashMap<String, String>,    // Query parameters from the request
}
```

For `ActionTarget::List`, `ids` contains all selected row IDs. For `ActionTarget::Detail`, `ids` contains the single record's ID.

### ActionResult

What the handler returns:

```rust
pub enum ActionResult {
    Success(String),   // Show a success flash message
    Redirect(String),  // Redirect to the given URL
    Error(String),     // Show an error flash message
}
```

## Building a Custom Action

Use `CustomAction::builder` to construct an action:

```rust
pub fn builder(name: &str, label: &str) -> CustomActionBuilder
```

### Builder Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `target` | `.target(ActionTarget)` | Where to render the button. Defaults to `ActionTarget::List`. |
| `confirm` | `.confirm(&str)` | Confirmation dialog message shown before running. |
| `icon` | `.icon(&str)` | Font Awesome class for the button icon. |
| `class` | `.class(&str)` | CSS class(es) for the button element. |
| `handler` | `.handler(F)` | **Required.** Async function that runs the action. Consumes the builder and returns `CustomAction`. |

### handler Signature

```rust
pub fn handler<F, Fut>(self, f: F) -> CustomAction
where
    F: Fn(ActionContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<ActionResult, AdminError>> + Send + 'static,
```

`.handler()` is the terminal method — it consumes the builder and produces the final `CustomAction`.

## Examples

### List Action (Bulk)

```rust
use axum_admin::entity::{CustomAction, ActionTarget, ActionContext, ActionResult};

let publish = CustomAction::builder("publish", "Publish Selected")
    .target(ActionTarget::List)
    .confirm("Publish all selected posts?")
    .icon("fa-solid fa-globe")
    .handler(|ctx: ActionContext| async move {
        for id in &ctx.ids {
            // publish logic...
        }
        Ok(ActionResult::Success(format!("Published {} posts", ctx.ids.len())))
    });
```

### Detail Action (Single Record)

```rust
let send_email = CustomAction::builder("send_welcome", "Send Welcome Email")
    .target(ActionTarget::Detail)
    .icon("fa-solid fa-envelope")
    .handler(|ctx: ActionContext| async move {
        let id = ctx.ids.first().cloned().unwrap_or_default();
        // send email for `id`...
        Ok(ActionResult::Success("Email sent".to_string()))
    });
```

### Redirect on Completion

```rust
let archive = CustomAction::builder("archive", "Archive")
    .target(ActionTarget::Detail)
    .confirm("Archive this record?")
    .handler(|ctx: ActionContext| async move {
        // perform archive...
        Ok(ActionResult::Redirect("/admin/posts".to_string()))
    });
```

## Registering Actions

Attach actions to an entity with `.action()`:

```rust
EntityAdmin::from_entity::<posts::Entity>("posts")
    .action(publish)
    .action(send_email)
    .action(archive)
```

Multiple actions can be registered on the same entity. They appear in the order they are registered.
