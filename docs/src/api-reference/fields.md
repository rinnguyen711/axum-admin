# Fields

`Field` is the core building block for admin forms and list views. Each field has a name, a label, a type, and optional modifiers.

---

## Field Constructors

All constructors take a `name: &str` and return a `Field`. The label defaults to the name with underscores replaced by spaces and the first letter capitalised.

| Constructor | FieldType set | Notes |
|---|---|---|
| `Field::text(name)` | `Text` | Single-line text input |
| `Field::textarea(name)` | `TextArea` | Multi-line text input |
| `Field::email(name)` | `Email` | Auto-wires `EmailFormat` validator |
| `Field::password(name)` | `Password` | Rendered as `<input type="password">` |
| `Field::number(name)` | `Number` | Integer input |
| `Field::float(name)` | `Float` | Decimal input |
| `Field::boolean(name)` | `Boolean` | Checkbox |
| `Field::date(name)` | `Date` | Date picker |
| `Field::datetime(name)` | `DateTime` | Date + time picker |
| `Field::json(name)` | `Json` | JSON editor textarea |
| `Field::select(name, options)` | `Select(Vec<(String, String)>)` | `options` is `Vec<(value, label)>` |
| `Field::foreign_key(name, label, adapter, value_field, label_field)` | `ForeignKey` | Dropdown populated by a `DataAdapter` |
| `Field::many_to_many(name, adapter)` | `ManyToMany` | Multi-select backed by a `ManyToManyAdapter` |
| `Field::file(name, storage)` | `File` | File upload; `storage` is `Arc<dyn FileStorage>` |
| `Field::image(name, storage)` | `Image` | Image upload; server validates `image/*` MIME type |
| `Field::custom(name, widget)` | `Custom(Box<dyn Widget>)` | Fully custom HTML widget |

---

## Builder Methods

All builder methods consume `self` and return `Self` so they can be chained.

### Appearance / visibility

| Method | Effect |
|---|---|
| `.label(label: &str)` | Override the display label |
| `.help_text(text: &str)` | Show help text below the input |
| `.hidden()` | Hide from both list and form views |
| `.readonly()` | Show value but disable editing |
| `.list_only()` | Show in list view only, not in forms |
| `.form_only()` | Show in forms only, not in list view |

### Validation

| Method | Adds validator |
|---|---|
| `.required()` | Sets `required = true` and adds `Required` validator |
| `.min_length(n: usize)` | `MinLength(n)` |
| `.max_length(n: usize)` | `MaxLength(n)` |
| `.min_value(n: f64)` | `MinValue(n)` — numeric fields |
| `.max_value(n: f64)` | `MaxValue(n)` — numeric fields |
| `.regex(pattern: &str)` | `RegexValidator::new(pattern)` — panics on invalid regex |
| `.unique(adapter, col: &str)` | Async `Unique` validator via `DataAdapter` |
| `.validator(v: Box<dyn Validator>)` | Add any custom sync validator |
| `.async_validator(v: Box<dyn AsyncValidator>)` | Add any custom async validator |

### ForeignKey options

| Method | Effect |
|---|---|
| `.fk_limit(n: u64)` | Limit the number of options loaded |
| `.fk_order_by(field: &str)` | Sort options by this column |

### File field options

| Method | Effect |
|---|---|
| `.accept(types: Vec<String>)` | Restrict accepted MIME types, e.g. `vec!["application/pdf".into()]`. No-op on non-File fields. |

---

## FieldType Enum

```rust
pub enum FieldType {
    Text,
    TextArea,
    Email,
    Password,
    Number,
    Float,
    Boolean,
    Date,
    DateTime,
    Select(Vec<(String, String)>),
    ForeignKey {
        adapter: Box<dyn DataAdapter>,
        value_field: String,
        label_field: String,
        limit: Option<u64>,
        order_by: Option<String>,
    },
    ManyToMany {
        adapter: Box<dyn ManyToManyAdapter>,
    },
    File {
        storage: Arc<dyn FileStorage>,
        accept: Vec<String>, // empty = any type accepted
    },
    Image {
        storage: Arc<dyn FileStorage>,
    },
    Json,
    Custom(Box<dyn Widget>),
}
```

---

## Widget Trait

Implement `Widget` to fully control how a field is rendered in forms and list views.

```rust
pub trait Widget: Send + Sync {
    /// Render the form input HTML for this field.
    fn render_input(&self, name: &str, value: Option<&str>) -> String;
    /// Render the display value for list/detail views.
    fn render_display(&self, value: Option<&str>) -> String;
}
```

Use `Field::custom(name, Box::new(MyWidget))` to attach a custom widget.

---

## Built-in Validators

All validators are in the `axum_admin::validator` module and re-exported at the crate root.

### Sync validators (`Validator` trait)

| Type | Triggered by | Behaviour |
|---|---|---|
| `Required` | `.required()` | Fails on empty/whitespace-only input |
| `MinLength(usize)` | `.min_length(n)` | Fails if `value.len() < n` |
| `MaxLength(usize)` | `.max_length(n)` | Fails if `value.len() > n` |
| `MinValue(f64)` | `.min_value(n)` | Fails if parsed f64 `< n`; skips empty values |
| `MaxValue(f64)` | `.max_value(n)` | Fails if parsed f64 `> n`; skips empty values |
| `RegexValidator` | `.regex(pattern)` | Fails if value does not match the pattern; skips empty |
| `EmailFormat` | `Field::email()` | Checks for `local@domain.tld` structure; skips empty |

### Async validators (`AsyncValidator` trait)

| Type | Triggered by | Behaviour |
|---|---|---|
| `Unique` | `.unique(adapter, col)` | Queries the adapter for conflicting rows; excludes current record on edit |

### Custom validators

```rust
pub trait Validator: Send + Sync {
    fn validate(&self, value: &str) -> Result<(), String>;
}

#[async_trait]
pub trait AsyncValidator: Send + Sync {
    async fn validate(&self, value: &str, record_id: Option<&Value>) -> Result<(), String>;
}
```

Attach with `.validator(Box::new(MyValidator))` or `.async_validator(Box::new(MyAsyncValidator))`.

---

## Example

```rust
use axum_admin::Field;

fn fields() -> Vec<Field> {
    vec![
        Field::text("title").required().max_length(200),
        Field::email("email").required().unique(adapter.clone(), "email"),
        Field::select("status", vec![
            ("draft".into(), "Draft".into()),
            ("published".into(), "Published".into()),
        ]),
        Field::foreign_key("author_id", "Author", Box::new(UserAdapter), "id", "name")
            .fk_order_by("name")
            .fk_limit(100),
        Field::boolean("active").help_text("Uncheck to deactivate this record."),
        Field::datetime("created_at").readonly().list_only(),
    ]
}
```
