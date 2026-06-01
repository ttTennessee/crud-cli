# JSON Entity Input

> **Other languages:** [简体中文](zh-CN/json-entity-input.md)

How to write entity JSON for `crud-cli gen --file <path>.json`.

The schema uses `deny_unknown_fields`: only the properties listed below are allowed at each level.

---

## Quick start

```bash
crud-cli gen --file entity.json
crud-cli gen --file entity.json --name User --package com.acme.app --table sys_user \
  --table-comment "System users" --var has_import=true
```

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

If your template bundle uses `generateWhen` / `skipWhen`, run `crud-cli validate` before `gen`.

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

Do **not** add other top-level keys. Do **not** set `is_sub` in JSON — include a `sub` object when you need master–detail.

---

## `sub` object (master–detail)

| Property | Required | Description |
|----------|----------|-------------|
| `name` | yes | Sub-entity name (e.g. `OrderItem`) |
| `table` | yes | Sub-table name |
| `fk_field` | yes | FK column on the sub table (e.g. `order_id`) |
| `fields` | yes | Array of field objects (same shape as master `fields`) |
| `table_comment` | no | Description of the sub table |

Master–detail is only supported with `--file`, not with `--fields`.

---

## Field objects (`fields` / `sub.fields`)

| Property | Required | Description |
|----------|----------|-------------|
| `name` | yes | Column name: starts with a letter; only letters, digits, `_` |
| `type` | yes | Type from the template’s `_field_types.toml` (see below) |
| `is_pk` | no | Primary key (default `false`) |
| `nullable` | no | Nullable |
| `comment` | no | Label / comment |
| `length` | no | Length (DDL, etc.) |
| `unique` | no | Unique |
| `default` | no | Default (any JSON value) |
| `extra` | no | Template-specific flags (see below) |

**Reserved `name` values** — do not use these as column names:  
`model`, `table`, `table_comment`, `package`, `package_path`, `fields`, and other names reserved by the tool (you’ll get a validation error if you do).

Mark exactly one master field with `"is_pk": true` when possible so primary-key metadata is inferred correctly.

---

## `variables` object

Templates declare which switches they need in `_variables.toml` inside the active template bundle. Your JSON may only set keys that appear there.

```json
"variables": {
  "module_name": "system",
  "function_name": "User management",
  "has_import": true
}
```

- Read `_variables.toml` for allowed keys, types (`bool` | `string` | `number`), and defaults.
- **Priority:** CLI `--var` > JSON `variables` > schema default.
- Unknown keys are rejected.

---

## Field `type`

Every `type` must be a canonical name or alias from `_field_types.toml` in the active template directory:

| Project setup | Template directory |
|---------------|-------------------|
| `[project] template = "name@version"` in `.crud/setup.toml` | `~/.crud/templates/<name>/<version>/` |
| No template pin | `.crud/templates/` |

---

## Field `extra`

Only needed when your template documents extra keys (e.g. RuoYi). Typical examples:

| Key | Type | Meaning |
|-----|------|---------|
| `query` | bool | Include in search/query |
| `query_like` | bool | LIKE query |
| `query_between` | bool | Range query |
| `list` | bool | List / export column |
| `insert` | bool | Create/edit form |
| `required` | bool | Required on form |
| `is_super` | bool | Field already on base entity |
| `auto_increment` | bool | Auto-increment PK |
| `dict_type` | string | Dictionary type |
| `read_converter_exp` | string | Excel converter |
| `is_datetime`, `is_textarea`, … | bool | Widget type |
| `ts_type` | string | TypeScript type override |

```json
{
  "name": "status",
  "type": "int",
  "comment": "Status",
  "extra": { "query": true, "list": true, "dict_type": "sys_normal_disable" }
}
```

Use only keys your template defines; unknown keys in `extra` are passed through but may be ignored.

---

## Master–detail example

```json
{
  "name": "Order",
  "table": "biz_order",
  "package": "com.acme.demo",
  "table_comment": "Order",
  "fields": [
    { "name": "order_id", "type": "Long", "is_pk": true, "comment": "Order PK" }
  ],
  "sub": {
    "name": "OrderItem",
    "table": "biz_order_item",
    "table_comment": "Order line items",
    "fk_field": "order_id",
    "fields": [
      { "name": "item_id", "type": "Long", "is_pk": true, "comment": "Line PK" },
      { "name": "order_id", "type": "Long", "comment": "Order FK" }
    ]
  },
  "variables": {
    "module_name": "business",
    "function_name": "Order management",
    "permission_prefix": "business:order"
  }
}
```

---

## CLI overrides

| Flag | Overrides |
|------|-----------|
| `--name` | `name` |
| `--package` | `package` |
| `--table` | `table` |
| `--table-comment` | `table_comment` |
| `--var key=value` | `variables[key]` |

`fields`, `sub`, and per-field data are **JSON only**.

---

## Common errors

| Message | What to check |
|---------|----------------|
| `unknown field` | Property name typo or key not allowed at that level |
| `unsupported` (field type) | `type` not listed in `_field_types.toml` |
| `undeclared variable` | Key in `variables` / `--var` missing from `_variables.toml` |
| `reserved_field_name` | Field `name` clashes with a reserved identifier |
| `variable shadows built-in` | `variables` key reserved by the tool — pick a name from `_variables.toml` only |
| Files skipped, no error | Run `validate`; template condition referred to a missing variable |

---

## See also

- [README.md](../README.md) — CLI overview and template helpers (for template authors)
- [Documentation index](./README.md)
