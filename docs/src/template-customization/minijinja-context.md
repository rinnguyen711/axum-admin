# MiniJinja Context Variables

Each template receives a context object serialized as a MiniJinja value. The variables available depend on which template is being rendered.

## Common Variables (List and Form Pages)

These variables are present in both `list.html` and `form.html`:

| Variable | Type | Description |
|---|---|---|
| `admin_title` | string | The title set via `.title("My Admin")` |
| `admin_icon` | string | Font Awesome class for the sidebar logo icon |
| `nav` | array | Sidebar navigation items (see below) |
| `current_entity` | string | Name of the currently active entity |
| `entity_name` | string | URL slug of the current entity |
| `entity_label` | string | Human-readable name of the current entity |
| `flash_success` | string or null | Success flash message |
| `flash_error` | string or null | Error flash message |
| `show_auth_nav` | bool | Whether to show Users / Roles links in the sidebar |

### Navigation (`nav`)

The `nav` array contains items of two types, distinguished by a `type` field:

**Entity link** (`type == "entity"`):

| Field | Type | Description |
|---|---|---|
| `name` | string | Entity URL slug |
| `label` | string | Display label |
| `icon` | string | Font Awesome class |
| `group` | string or null | Group name if grouped |

**Group** (`type == "group"`):

| Field | Type | Description |
|---|---|---|
| `label` | string | Group display label |
| `entities` | array | Child entity links (same shape as entity type) |
| `active` | bool | True if any child is currently active |

## List Page (`list.html`, `list_table.html`)

| Variable | Type | Description |
|---|---|---|
| `columns` | array of strings | Column names to display |
| `column_types` | map | Maps column name to field type string |
| `rows` | array | Row data (see below) |
| `search` | string | Current search query |
| `page` | integer | Current page number |
| `total_pages` | integer | Total number of pages |
| `order_by` | string | Column currently sorted |
| `order_dir` | string | `"asc"` or `"desc"` |
| `filter_fields` | array | Fields available as filters (see FieldContext below) |
| `active_filters` | map | Currently applied filter values, keyed by field name |
| `actions` | array | Bulk/row actions (see ActionContext below) |
| `bulk_delete` | bool | Whether bulk delete is enabled |
| `bulk_export` | bool | Whether bulk CSV export is enabled |
| `export_columns` | array of `[name, label]` pairs | Columns available for export |
| `can_create` | bool | Whether the current user can create records |
| `can_edit` | bool | Whether the current user can edit records |
| `can_delete` | bool | Whether the current user can delete records |

### Row (`rows[n]`)

| Field | Type | Description |
|---|---|---|
| `id` | string | Record primary key |
| `data` | map | Column name → display value (all strings) |

### ActionContext (`actions[n]`)

| Field | Type | Description |
|---|---|---|
| `name` | string | Action identifier used in the POST URL |
| `label` | string | Button label |
| `target` | string | `"list"` (bulk) or `"row"` (per-row) |
| `confirm` | string or null | Confirmation message, if any |
| `icon` | string or null | Font Awesome class for button icon |
| `class` | string or null | Custom CSS classes for the button |

## Form Page (`form.html`)

| Variable | Type | Description |
|---|---|---|
| `fields` | array | Field definitions (see FieldContext below) |
| `values` | map | Current field values, keyed by field name |
| `errors` | map | Validation error messages, keyed by field name |
| `is_create` | bool | True when creating a new record, false when editing |
| `record_id` | string | The record's primary key (empty string on create) |
| `csrf_token` | string | CSRF token to include in form submissions |
| `can_save` | bool | Whether the current user can save this record |

### FieldContext (`fields[n]`)

| Field | Type | Description |
|---|---|---|
| `name` | string | Field name (used as `<input name>`) |
| `label` | string | Human-readable label |
| `field_type` | string | One of: `Text`, `Textarea`, `Integer`, `Float`, `Boolean`, `Select`, `Date`, `DateTime`, `File`, `ManyToMany`, `Hidden` |
| `readonly` | bool | Field is read-only |
| `hidden` | bool | Field is hidden entirely |
| `list_only` | bool | Field only appears on list pages |
| `form_only` | bool | Field only appears on form pages |
| `required` | bool | Field is required |
| `help_text` | string or null | Optional help text shown below the field |
| `options` | array of `[value, label]` pairs | Options for `Select` fields |
| `selected_ids` | array of strings | Currently selected IDs for `ManyToMany` fields |
| `accept` | array of strings | Accepted MIME types for `File` fields |

## Login Page (`login.html`)

| Variable | Type | Description |
|---|---|---|
| `admin_title` | string | Admin panel title |
| `error` | string or null | Login error message |
| `csrf_token` | string | CSRF token |
| `next` | string or null | URL to redirect to after login |

## Flash Partial (`flash.html`)

| Variable | Type | Description |
|---|---|---|
| `success` | string or null | Success message |
| `error` | string or null | Error message |
