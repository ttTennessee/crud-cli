# crud-cli JSON entity input spec

<!-- TODO: this file is currently a copy of docs/entity.md.
     The agent-facing version will be rewritten to add LLM-only prompt content
     (e.g. up-front field-shape clarifications: input vs. select, dictionary
     code confirmation). Until then the two files have identical content. -->

Schema for the `entity.json` accepted by `crud-cli gen --file <path>` and by the MCP `crud_preview` / `crud_generate` tools.

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
