# JSON Entity Input

> **Other languages:** [简体中文](zh-CN/json-entity-input.md)

The `entity.json` you build for the MCP `crud_preview` / `crud_generate` tools (also accepted by `--file`).

The schema uses `deny_unknown_fields`: only the properties listed below are allowed at each level.

Minimal example:

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "Primary key" },
    { "name": "email", "type": "String", "comment": "Email" }
  ]
}
```

---

## Top-level object

| Property | Required | Description |
|----------|----------|-------------|
| `name` | yes | Entity/class name (e.g. `User`, `Order`) |
| `table` | yes | Master table name |
| `package` | yes | Server package (e.g. Java) |
| `fields` | yes | Array of field objects (below) |
| `table_comment` | no | Description of the master table |
| `sub` | no | Master–detail block (below) |
| `variables` | no | Values for keys declared in the template’s `_variables.toml` |

Do **not** add other top-level keys. Do **not** set `is_sub` — include a `sub` object when you need master–detail (`--fields` cannot express `sub`).

## `sub` object (master–detail)

Same shape as the top level, plus a foreign key:

| Property | Required | Description |
|----------|----------|-------------|
| `name` | yes | Sub-entity name (e.g. `OrderItem`) |
| `table` | yes | Sub-table name |
| `fk_field` | yes | FK column on the sub table (e.g. `order_id`) |
| `fields` | yes | Field objects (same shape as master `fields`) |
| `table_comment` | no | Description of the sub table |

## Field objects (`fields` / `sub.fields`)

| Property | Required | Description |
|----------|----------|-------------|
| `name` | yes | Column name: starts with a letter; only letters, digits, `_`. Cannot be a reserved identifier (`model`, `table`, `table_comment`, `package`, `package_path`, `fields`) |
| `type` | yes | A canonical name or alias from the active template’s `_field_types.toml` (resource `crud://templates/field-types`) |
| `is_pk` | no | Primary key (default `false`; mark exactly one master field) |
| `required` | no | Required (default `false`) |
| `comment` | no | Label / comment |
| `length` | no | Length (DDL, etc.) |
| `unique` | no | Unique |
| `default` | no | Default (any JSON value) |
| `extra` | no | Template-specific flags (below) |

## `variables` object

Top-level switches the template declares in `_variables.toml` (resource `crud://templates/variables`). Only keys that appear there are allowed; unknown keys are rejected. Read it for allowed types (`bool` | `string` | `number`) and defaults.

```json
"variables": { "module_name": "system", "has_import": true }
```

## `extra` object

Per-field flags, only meaningful when the template documents them (e.g. RuoYi: `query`, `list`, `insert`, `dict_type`, `ts_type`, …). Keys the template does not use are passed through and may be ignored.

```json
{ "name": "status", "type": "int", "extra": { "query": true, "dict_type": "sys_normal_disable" } }
```

## Master–detail example

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

---

## Common errors

| Message | What to check |
|---------|----------------|
| `unknown field` | Property name typo or key not allowed at that level |
| `unsupported` (field type) | `type` not in `_field_types.toml` |
| `undeclared variable` | Key in `variables` missing from `_variables.toml` |
| `reserved_field_name` | Field `name` clashes with a reserved identifier |
| `variable shadows built-in` | `variables` key reserved by the tool |

## See also

- [MCP Server](mcp-server.md) — `crud_preview` / `crud_generate` tools and `crud://` resources
- [Documentation index](./README.md)
