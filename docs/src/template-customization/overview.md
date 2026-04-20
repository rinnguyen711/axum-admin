# Template Customization Overview

axum-admin renders its UI using [MiniJinja](https://docs.rs/minijinja), a Rust implementation of the Jinja2 template engine. All built-in templates are compiled into the binary, so the admin works out of the box with no files on disk — but you can override any of them at startup.

## How Template Resolution Works

Templates are resolved in this priority order (highest to lowest):

1. **Inline overrides** — templates registered with `.template(name, content)`
2. **Directory overrides** — templates loaded from a directory via `.template_dir(path)`
3. **Built-in templates** — compiled into the binary at build time

When more than one `.template_dir()` call is made, later calls take precedence over earlier ones. Inline `.template()` calls always win regardless of order.

Templates are loaded once at startup. There is no runtime reloading.

## Override Methods

### `.template(name, content)`

Registers a single template by name from a string. Use this for small overrides or when you want to embed template content in your Rust source.

```rust
AdminApp::new()
    .template("home.html", include_str!("templates/my_home.html"))
    // ...
```

The `name` must match the built-in template filename exactly (e.g. `"layout.html"`, `"list.html"`). You can also register entirely new template names for use with custom routes.

### `.template_dir(path)`

Loads all `.html` files from a directory. Any filename that matches a built-in template overrides it; files with new names are added as additional templates.

```rust
AdminApp::new()
    .template_dir("templates/admin")
    // ...
```

Multiple directories can be registered. Later calls take precedence:

```rust
AdminApp::new()
    .template_dir("templates/base")    // lower priority
    .template_dir("templates/custom")  // higher priority
    // ...
```

## Built-in Template Files

The following templates are compiled into the binary and can be overridden by name:

| Template | Used for |
|---|---|
| `layout.html` | Outer HTML shell, sidebar, header |
| `home.html` | Dashboard / index page |
| `list.html` | Entity list page (search, filters, bulk actions) |
| `list_table.html` | The table portion of the list page (rendered via HTMX) |
| `form.html` | Create / edit form |
| `flash.html` | Flash message partial |
| `login.html` | Login page |
| `change_password.html` | Change password page |
| `users_list.html` | Built-in users management list |
| `user_form.html` | Built-in user create/edit form |
| `roles.html` | Built-in roles list |
| `role_form.html` | Built-in role create form |
| `role_edit_form.html` | Built-in role edit form |
| `forbidden.html` | 403 forbidden page |

## Template Engine: MiniJinja

All templates use [MiniJinja](https://docs.rs/minijinja) syntax, which is compatible with Jinja2:

- `{{ variable }}` — output a variable
- `{% if condition %}...{% endif %}` — conditionals
- `{% for item in list %}...{% endfor %}` — loops
- `{% extends "layout.html" %}` / `{% block name %}...{% endblock %}` — inheritance (used by built-in templates)
- `{% include "partial.html" %}` — includes

The built-in templates use `{% extends "layout.html" %}` with a `{% block content %}` block. When overriding a page template (e.g. `list.html`), you can extend `layout.html` the same way.
