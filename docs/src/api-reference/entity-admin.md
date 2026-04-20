# EntityAdmin

`EntityAdmin` and `EntityGroupAdmin` are the core types for registering entities with the admin panel.

---

## EntityAdmin

Represents a single data model in the admin panel — its list view, form fields, search configuration, custom actions, and access permissions.

```rust
use axum_admin::{EntityAdmin, Field, FieldType};

let posts = EntityAdmin::new::<()>("posts")
    .label("Blog Posts")
    .icon("fa-solid fa-newspaper")
    .adapter(Box::new(my_adapter))
    .field(Field::new("title", FieldType::Text))
    .field(Field::new("body", FieldType::Textarea))
    .list_display(vec!["title".into(), "created_at".into()])
    .search_fields(vec!["title".into(), "body".into()]);
```

---

### Constructors

#### `EntityAdmin::new<T>(entity: &str) -> Self`

Creates a new `EntityAdmin` for the given entity name. The type parameter `T` is a marker and can be `()` when not using SeaORM entity inference.

Defaults:

| Field | Default |
|---|---|
| `label` | Auto-generated from entity name (e.g. `"blog_posts"` → `"Blog Posts"`) |
| `icon` | `"fa-solid fa-layer-group"` |
| `pk_field` | `"id"` |
| `bulk_delete` | `true` |
| `bulk_export` | `true` |

---

#### `EntityAdmin::from_entity<E>(name: &str) -> Self`

*(Requires `seaorm` feature)*

Creates an `EntityAdmin` with fields inferred from a SeaORM entity type. Field types are derived automatically from column definitions.

```rust
EntityAdmin::from_entity::<entity::post::Entity>("posts")
```

---

### Builder Methods

#### `label(self, label: &str) -> Self`

Overrides the human-readable label shown in the sidebar and page titles.

```rust
.label("Blog Posts")
```

---

#### `icon(self, icon: &str) -> Self`

Sets the Font Awesome icon class for this entity in the sidebar and dashboard. Defaults to `"fa-solid fa-layer-group"`.

```rust
.icon("fa-solid fa-newspaper")
```

---

#### `pk_field(self, pk: &str) -> Self`

Overrides the primary key field name. Defaults to `"id"`.

```rust
.pk_field("uuid")
```

---

#### `group(self, group: &str) -> Self`

Assigns this entity to a named sidebar group. Entities sharing the same group label are collapsed under a single expandable section. Using `EntityGroupAdmin` is usually more ergonomic for this.

```rust
.group("Content")
```

---

#### `adapter(self, adapter: Box<dyn DataAdapter>) -> Self`

Sets the data adapter responsible for list, create, update, and delete operations.

```rust
.adapter(Box::new(PostAdapter { db: db.clone() }))
```

---

#### `field(self, field: Field) -> Self`

Adds a field to the form and/or list. If a field with the same name already exists, it is replaced. Can be called multiple times.

```rust
.field(Field::new("title", FieldType::Text))
.field(Field::new("status", FieldType::Select(options)))
```

---

#### `list_display(self, fields: Vec<String>) -> Self`

Sets the list of field names shown as columns in the entity list view. If empty, all fields are shown.

```rust
.list_display(vec!["title".into(), "status".into(), "created_at".into()])
```

---

#### `search_fields(self, fields: Vec<String>) -> Self`

Configures which field names are searched when a search query is submitted on the list page.

```rust
.search_fields(vec!["title".into(), "body".into()])
```

---

#### `filter_fields(self, fields: Vec<String>) -> Self`

Specifies the field names available as sidebar filters on the list page. Uses the field definitions already registered via `.field()`.

```rust
.filter_fields(vec!["status".into(), "category".into()])
```

---

#### `filter(self, field: Field) -> Self`

Adds or replaces a dedicated filter field. Use this when the filter control should differ from the form field (e.g. a Select filter for a Text form field). If a filter with the same name already exists, it is replaced.

```rust
.filter(Field::new("status", FieldType::Select(status_options)))
```

---

#### `bulk_delete(self, enabled: bool) -> Self`

Enables or disables the bulk-delete action on the list page. Defaults to `true`.

```rust
.bulk_delete(false)
```

---

#### `bulk_export(self, enabled: bool) -> Self`

Enables or disables the bulk CSV export action on the list page. Defaults to `true`.

```rust
.bulk_export(false)
```

---

#### `action(self, action: CustomAction) -> Self`

Registers a custom action button. Use `CustomAction::builder()` to construct the action. Can be called multiple times.

```rust
.action(
    CustomAction::builder("publish", "Publish")
        .target(ActionTarget::List)
        .confirm("Publish selected posts?")
        .icon("fa-solid fa-upload")
        .handler(|ctx| async move {
            // ...
            Ok(ActionResult::Success("Published.".into()))
        })
        .build()
)
```

---

#### `before_save<F>(self, f: F) -> Self`

Registers a synchronous hook called before a record is created or updated. Receives a mutable reference to the form data map. Return `Err(AdminError)` to abort the save.

```rust
where F: Fn(&mut HashMap<String, Value>) -> Result<(), AdminError> + Send + Sync + 'static
```

```rust
.before_save(|data| {
    data.insert("updated_at".into(), Value::String(Utc::now().to_rfc3339()));
    Ok(())
})
```

---

#### `after_delete<F>(self, f: F) -> Self`

Registers a synchronous hook called after a record is deleted. Receives the deleted record's primary key value. Return `Err(AdminError)` to surface an error (record is already deleted).

```rust
where F: Fn(&Value) -> Result<(), AdminError> + Send + Sync + 'static
```

```rust
.after_delete(|id| {
    println!("Deleted record: {id}");
    Ok(())
})
```

---

### Permission Methods

These methods restrict access to specific operations on the entity. Permission strings are checked against the authenticated user's roles via the configured enforcer.

#### `require_view(self, perm: &str) -> Self`

Requires `perm` to list records or open the edit form.

#### `require_create(self, perm: &str) -> Self`

Requires `perm` to create a new record.

#### `require_edit(self, perm: &str) -> Self`

Requires `perm` to submit an edit.

#### `require_delete(self, perm: &str) -> Self`

Requires `perm` to delete a record.

#### `require_role(self, role: &str) -> Self`

Shortcut: sets the same permission string for all four operations (view, create, edit, delete).

```rust
.require_role("admin")
// equivalent to:
.require_view("admin")
.require_create("admin")
.require_edit("admin")
.require_delete("admin")
```

---

## EntityGroupAdmin

Groups multiple `EntityAdmin` instances under a collapsible sidebar section. Register the group with `AdminApp::register()` the same way as a plain `EntityAdmin`.

```rust
use axum_admin::EntityGroupAdmin;

let content_group = EntityGroupAdmin::new("Content")
    .icon("fa-solid fa-folder")
    .register(posts_entity)
    .register(pages_entity);

AdminApp::new()
    // ...
    .register(content_group)
```

---

### Constructor

#### `EntityGroupAdmin::new(label: &str) -> Self`

Creates a new group with the given sidebar label.

---

### Builder Methods

#### `icon(self, icon: &str) -> Self`

Sets an optional Font Awesome icon shown next to the group label in the sidebar.

```rust
.icon("fa-solid fa-folder-open")
```

---

#### `register(self, entity: EntityAdmin) -> Self`

Adds an `EntityAdmin` to this group. Can be called multiple times. When the group is registered with `AdminApp::register()`, all member entities are stamped with the group label.

```rust
.register(posts_entity)
.register(comments_entity)
```
