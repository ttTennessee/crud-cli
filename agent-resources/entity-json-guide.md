# crud-cli JSON entity input spec

Schema for the `entity.json` accepted by `crud-cli gen --file <path>` and by the MCP `crud_preview` / `crud_generate` tools.

## Before you write the JSON

Resolve the following points **before** emitting `entity.json`. Only ask the user about items that are genuinely ambiguous — do **not** confirm fields whose shape is obvious (e.g. `name` is clearly a `varchar` text input, `age` is clearly a small integer). Ask once, in a single consolidated message, only about the grey-area items below.

1. **Dictionary-backed fields.** If a field maps to an enumerated value (status, type, category, gender, level, …), confirm **which existing dictionary code in the current system** it should bind to. Do not invent a new dictionary code or guess at one that "sounds right." If you cannot find an existing code that fits, ask the user for the exact `dict_type` string.
2. **Frontend widget for non-trivial fields.** If the active template renders a frontend page, confirm the input widget for fields where more than one choice is plausible: `input`, `input-number`, `textarea`, `select`, `radio`, `checkbox`, `switch`, `date-picker`, `datetime-picker`, `time-picker`, `upload`, `editor`. Plain short text (`name`, `title`, `code`) defaults to `input` and does not need confirmation; long descriptive text, numeric quantities, dictionary-backed values, booleans, and date/time fields usually do.
3. **List vs. detail vs. query visibility.** When the template distinguishes list-column / query-condition / insert-form / edit-form visibility (RuoYi-style `list` / `query` / `insert` / `edit` flags), confirm any field whose default is non-obvious — e.g. large text bodies typically excluded from list columns, audit fields excluded from forms, status-like fields usually included as query conditions.
4. **Required / unique constraints.** Confirm `required` and `unique` only when the business meaning is ambiguous. The PK is obviously required; a column literally named `email` or `username` is almost certainly unique — do not ask.
5. **Numeric precision.** For money / decimal / large-integer fields, confirm precision and scale (or the canonical type alias) when the template offers more than one numeric option. Default integer / string types do not need confirmation.
6. **Template variables.** If the active template's `_variables.toml` declares switches (e.g. `module_name`, `permission_prefix`, `has_import`), confirm the values when they cannot be inferred from the entity name or package.

Rule of thumb: **silence on the obvious, one consolidated question on the grey areas.** Never ask the user to re-state information already present in the prompt (entity name, package, table name, plainly-typed columns).

All objects use `deny_unknown_fields` — any key not listed below is rejected.

## Top-level

| Key | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Entity / class name (e.g. `User`) |
| `table` | string | yes | Master table physical name |
| `package` | string | yes | Server package (e.g. Java `com.acme.demo`) |
| `fields` | FieldSpec[] | yes | Master-table fields |
| `table_comment` | string | no | Description of master table |
| `sub` | SubSpec | no | Master–detail block; presence implies `is_sub = true` (do not set `is_sub` yourself) |
| `variables` | object | no | Values for keys declared in the active template's `_variables.toml` |

## `SubSpec`

| Key | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Sub-entity name (e.g. `OrderItem`) |
| `table` | string | yes | Sub-table name |
| `fk_field` | string | yes | FK column on sub-table (e.g. `order_id`) |
| `fields` | FieldSpec[] | yes | Sub-table fields |
| `table_comment` | string | no | Description of sub-table |

## `FieldSpec`

Used in both `fields` and `sub.fields`.

| Key | Type | Required | Notes |
|---|---|---|---|
| `name` | string | yes | Column name. Must start with a letter; only `[A-Za-z0-9_]`. Cannot be one of the reserved identifiers: `model`, `table`, `table_comment`, `package`, `package_path`, `fields` |
| `type` | string | yes | A canonical name or alias from the active template's `_field_types.toml` (also returned by the MCP `crud_describe_templates` tool) |
| `is_pk` | bool | no | Default `false`. Mark exactly one master-table field as PK |
| `required` | bool | no | Default `false` |
| `comment` | string | no | Label / column comment |
| `length` | number | no | DDL length |
| `unique` | bool | no | Unique constraint |
| `default` | any | no | Default value (any JSON type) |
| `extra` | object | no | Template-specific per-field flags (see below) |

## `variables` object

Top-level switches declared by the active template's `_variables.toml` (also returned by the MCP `crud_describe_templates` tool). Keys not declared there are rejected. Allowed value types: `bool` | `string` | `number`.

```json
"variables": { "module_name": "system", "has_import": true }
```

## `extra` object (per field)

Free-form per-field flags consumed by template extensions (e.g. RuoYi-style bundles use `query`, `list`, `insert`, `dict_type`, `ts_type`). Keys the active template does not consume are passed through and ignored.

```json
{ "name": "status", "type": "int", "extra": { "query": true, "dict_type": "sys_normal_disable" } }
```

## Examples

### Flat

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "Primary key" },
    { "name": "email", "type": "String", "length": 128, "unique": true, "comment": "Email" }
  ]
}
```

### Master–detail

```json
{
  "name": "Order",
  "table": "biz_order",
  "package": "com.acme.demo",
  "fields": [
    { "name": "order_id", "type": "Long", "is_pk": true, "comment": "Order PK" }
  ],
  "sub": {
    "name": "OrderItem",
    "table": "biz_order_item",
    "fk_field": "order_id",
    "fields": [
      { "name": "item_id", "type": "Long", "is_pk": true, "comment": "Line PK" },
      { "name": "order_id", "type": "Long", "comment": "Order FK" }
    ]
  },
  "variables": { "module_name": "business", "permission_prefix": "business:order" }
}
```

## Errors

| Message | Cause |
|---|---|
| `unknown field` | Key not allowed at that level (typo or extra key) |
| `unsupported` | `type` not in `_field_types.toml` |
| `undeclared variable` | Key in `variables` missing from `_variables.toml` |
| `reserved_field_name` | `FieldSpec.name` collides with reserved identifier |
| `variable shadows built-in` | `variables` key uses a built-in name |
