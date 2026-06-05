# crud-cli template authoring spec

**Languages:** English · [简体中文](zh-CN/templates.md)

Authoritative reference for writing Handlebars (`.hbs`) template bundles for `crud-cli`.

## Bundle layout

```
<bundle-root>/                   # project: .crud/templates/  |  global: ~/.crud/templates/<name>/<version>/
  **/*.hbs                       # templates; optional YAML front-matter
  _variables.toml                # per-invocation extended top-level variable schema
  _field_types.toml              # allowed field type names
  _field_extra.toml              # optional; declares valid keys for fields[].extra
  type_map.toml                  # optional; consumed by the ty_map helper
  .crudignore                    # exclude files from discovery
```

The first path segment of each template (e.g. `java/`, `vue/`, `resources/`) is a **prefix** resolved against `[paths.lang]` then `[paths.aux]` in `.crud/setup.toml` and rebased to the configured directory.

## Front-matter

Optional YAML block at the top of any `.hbs` file:

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only
generateWhen: has_import
---
```

| Key | Type | Notes |
|---|---|---|
| `basePath` / `base_path` | string | Output directory relative to project root. May reference built-in or extended top-level variables. |
| `filename` | string | Output file name. **Single path segment** — no `/`, no `..`. May reference variables. |
| `overwrite` | enum | `never` \| `force-only` \| `always`. Overrides global/user default. |
| `generateWhen` / `generate_when` | string | Generate only when condition is truthy. Value is the inner expression of `{{#if ...}}` — no surrounding `{{ }}`. |
| `skipWhen` / `skip_when` | string | Skip when condition is truthy. Mutually exclusive with `generateWhen`. |

**Truthiness**: `false`, missing, `""`, `0`, `[]` are falsy.

**Condition pitfall**: an undeclared variable in `generateWhen` / `skipWhen` evaluates falsy and **silently skips** the file. Always run `crud-cli validate` to surface typos.

Quote front-matter values containing `{{` to avoid YAML parse errors.

## Built-in top-level variables

Injected automatically on every `gen`. **Never declare these** in `_variables.toml` or `setup.toml` `[variables]` — triggers `variable shadows built-in`.

### Entity / table

| Variable | Type | Example |
|---|---|---|
| `model` | string | `User` |
| `model_pascal` | string | `User` |
| `model_snake` | string | `user` |
| `model_camel` | string | `user` |
| `model_kebab` | string | `user` |
| `table` | string | `sys_user` |
| `table_comment` | string | `""` if omitted |
| `package` | string | `com.acme.demo` |
| `package_path` | string | `com/acme/demo` |

### Primary key (derived from `fields` where `is_pk: true`)

| Variable | Type | Default when no PK |
|---|---|---|
| `pk_field` | string (camelCase) | `id` |
| `pk_field_type` | string | `Long` |
| `pk_field_pascal` | string | `Id` |

### Master–detail

| Variable | Type | When absent |
|---|---|---|
| `is_sub` | bool | `false` |
| `sub_table`, `sub_table_comment` | string | `""` |
| `sub_model`, `sub_model_snake`, `sub_model_pascal`, `sub_model_camel`, `sub_model_kebab` | string | `""` |
| `sub_model_fk` | string (camelCase FK column) | `""` |
| `sub_model_fk_pascal` | string | `""` |
| `sub_fields` | array | `[]` |

### Field arrays

`fields` and `sub_fields` — iterate with `{{#each}}`; per-item shape below.

### Author / time

| Variable | Source |
|---|---|
| `git_user_name`, `git_user_email` | git config |
| `user_name`, `user_email` | `.crud/setup.user.toml`; falls back to git when empty |
| `date` | `YYYY-MM-DD` (local) |
| `datetime` | `YYYY-MM-DD HH:MM:SS` (local) |
| `year` | four-digit year |

### Handlebars `{{#each}}` specials (recognised by validator)

`this`, `@index`, `@key`, `@first`, `@last`, `@root`.

## Field object shape (`{{#each fields}}` / `{{#each sub_fields}}`)

Default properties — present on every field:

| Property | Type | When omitted by input |
|---|---|---|
| `name` | string | — |
| `name_pascal`, `name_snake`, `name_camel`, `name_kebab` | string | — |
| `type` | string | — |
| `is_pk` | bool | `false` |
| `required` | bool | `false` |
| `comment` | string | `""` |
| `length` | number \| null | `null` |
| `unique` | bool | `false` |
| `default` | any \| null | `null` |

The `--fields` DSL populates only `name`, `type`, `is_pk`. For full metadata, use JSON input (see `entity` resource).

**Extended field properties**: a bundle's caller can pass extra key/value pairs in `fields[].extra` that are flattened into each field object — accessible inside `{{#each fields}}` at the same level as defaults.

Template authors declare these keys in `_field_extra.toml` (bundle root). Agents query them via the MCP `crud_describe_templates` tool (`field_extra` key in the response). `validate` does not statically recognise extension keys; a bundle-defined key triggering `unknown variable` is a spelling mismatch.

```toml
# _field_extra.toml
[options]
description = "Enum option list; each item is {label, value}"
type        = "array"
required_for = ["enum", "radio"]   # required when field type is enum or radio

[dict_type]
description = "Dictionary code binding for select/radio fields"
type        = "string"

[query]
description = "Include this field as a query condition in list pages"
type        = "bool"
```

`_field_extra.toml` fields per key:

| Field | Required | Notes |
|---|---|---|
| `description` | yes | Human/agent-readable purpose |
| `type` | yes | `string` \| `number` \| `bool` \| `array` \| `object` |
| `required_for` | no | Field types that require this key (empty = always optional) |

## Extended top-level variables

Custom variables for front-matter, `{{#if}}`, and bodies:

| Source | Purpose |
|---|---|
| `_variables.toml` (bundle root) | Schema: `type`, `default`, `required`, `description`. `description` is the contract callers read. |
| `.crud/setup.toml` `[variables]` | Project-level defaults |
| `--var key=value` | Per-call override |
| `variables` object in JSON gen input | Per-call override |

**Precedence**: `--var` > JSON `variables` > schema `default`.

Custom names **must not collide** with built-ins or with `fields` / `sub_fields`. `validate` accepts: built-ins ∪ `_variables.toml` declarations ∪ `[variables]` keys.

```toml
# _variables.toml
[has_import]
description = "Generate import button and importExcel endpoint"
type        = "bool"
default     = false

[module_name]
description = "Business module id for permission prefix and routes"
type        = "string"
required    = true
```

`_variables.toml` types: `bool` \| `string` \| `number`.

## Helpers

### Case conversion (one string arg)

| Helper | `hello_world` → |
|---|---|
| `pascal_case` | `HelloWorld` |
| `snake_case` | `hello_world` |
| `camel_case` | `helloWorld` |
| `kebab_case` | `hello-world` |

### Brace wrapping (no HTML escaping)

Output is **not** HTML-escaped — `<List<T>>` passes through. These helpers emit literal braces without conflicting with Handlebars syntax:

| Helper | With `name_camel = userId` | Output |
|---|---|---|
| `single_brace` | `{{single_brace name_camel}}` | `{userId}` |
| `double_brace` | `{{double_brace name_camel}}` | `{{userId}}` |

MyBatis (`#` / `$` go outside):

```handlebars
WHERE id = #{{single_brace pk_field}}
ORDER BY ${{single_brace pk_field}}
```

Vue literal interpolation:

```handlebars
<span>{{double_brace name_camel}}</span>
```

### `ty_map`

Maps neutral type names to target-stack types using `type_map.toml` in the current bundle:

```handlebars
private {{ty_map type}} {{name_camel}};
```

On miss, `.crud/setup.toml` `[type_map].fallback` governs behavior:

| `fallback` | Behavior |
|---|---|
| `passthrough` (default) | Emit type string unchanged |
| `error` | Abort render |
| any other string | Replace with that literal |

### Standard Handlebars (accepted by validator)

Block: `{{#if}}` / `{{#unless}}` / `{{#each}}` / `{{#with}}`.
Subexpressions: `(eq a b)`, `(ne a b)`, `(and a b)`, `(or a b)`, `(not x)`.
Paths: `../` for parent context; `lookup` for dynamic property access.

## Workflow

1. Read `.crud/setup.toml`, `_variables.toml`, `_field_types.toml`, and one or two **hand-written examples** of the same kind of file in the target project — they are the style ground truth.
2. Write `.hbs` files; declare extended variables in `_variables.toml`; route output via front-matter `basePath` / `filename`.
3. `crud-cli validate` — catches unknown variables, missing helpers, unsafe filenames.
4. `crud-cli gen ... --dry-run` then `--stdout` to compare against the hand-written example.
5. Iterate by **editing templates**, not by hand-fixing generated code.

## Error catalogue

| Message | Cause |
|---|---|
| `unknown variable` / `UnknownVariable` | Variable not in built-ins ∪ `_variables.toml` ∪ `[variables]` (or it's a bundle-defined extended field key — verify spelling against bundle docs) |
| `variable shadows built-in` | `_variables.toml` or `[variables]` uses a reserved built-in name |
| `missing_helper` | Helper not registered and not a Handlebars built-in |
| `helper not found` (render) | Same as above, or `ty_map` with `fallback=error` on an unmapped type |
| `[skipped: condition]` | `generateWhen` / `skipWhen` evaluated falsy |
| `invalid filename` | Front-matter `filename` has `/` or `..` |
| YAML parse error in front-matter | Quote values containing `{{` |
