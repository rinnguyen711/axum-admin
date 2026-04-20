# Overriding Templates

You can replace any built-in template with your own version. Templates must be valid MiniJinja and receive the same context variables described in the [MiniJinja Context](./minijinja-context.md) chapter.

## Choosing an Override Method

Use `.template()` for a single template you want to keep inline or load with `include_str!`:

```rust
AdminApp::new()
    .template("home.html", include_str!("../templates/admin/home.html"))
    // ...
```

Use `.template_dir()` when you're overriding several templates and prefer to keep them as files on disk:

```rust
AdminApp::new()
    .template_dir("templates/admin")
    // ...
```

Both methods can be combined. `.template()` always wins over `.template_dir()` regardless of call order.

## Example: Custom Home Page

Create `templates/admin/home.html`:

```html
{% extends "layout.html" %}
{% block title %}Dashboard — {{ admin_title }}{% endblock %}
{% block content %}
<div class="max-w-2xl">
  <h1 class="text-2xl font-bold text-zinc-900 mb-2">Welcome</h1>
  <p class="text-zinc-500">You are logged in to {{ admin_title }}.</p>
</div>
{% endblock %}
```

Register it in your app setup:

```rust
AdminApp::new()
    .title("My App")
    .template_dir("templates/admin")
    .auth(...)
    // ...
```

## Template Inheritance

The built-in templates use MiniJinja's `{% extends %}` / `{% block %}` system. `layout.html` defines two blocks:

- `{% block title %}` — the `<title>` tag content
- `{% block content %}` — the main page body inside the scrollable content area

When writing a replacement for any page template (e.g. `list.html`, `form.html`, `home.html`), extend `layout.html` to get the sidebar, header, and flash messages for free:

```html
{% extends "layout.html" %}
{% block title %}{{ entity_label }} — {{ admin_title }}{% endblock %}
{% block content %}
  <!-- your page content here -->
{% endblock %}
```

If you replace `layout.html` itself, the page templates that extend it will use your version automatically.

## Adding New Templates

You can add templates with names that don't correspond to any built-in. These are available for use in custom Axum routes that call the renderer directly, or via `{% include %}` from other templates:

```rust
AdminApp::new()
    .template("widgets/stat_card.html", include_str!("../templates/stat_card.html"))
    // ...
```

## Partial Overrides via `{% include %}`

The list page renders its table section by including `list_table.html`. Overriding `list_table.html` alone lets you change how rows are displayed without replacing the full list page (search bar, filters, bulk actions, etc.).

## Notes

- Template files are loaded once at startup. Editing files on disk after the server starts has no effect until restart.
- Template errors are caught at render time and return an HTML error message in development, so a broken override will not panic the server.
- The `basename` filter is available in all templates: `{{ path | basename }}` returns the filename portion of a path string.
