# Template Authoring

Authoritative spec for writing Handlebars (`.hbs`) template bundles for `crud-cli`. Served to LLM agents as the MCP prompt `crud_template_authoring`.

## Template bundle layout

A typical bundle contains:

| File / directory | Purpose |
|------------------|---------|
| `**/*.hbs` | Handlebars templates; optional YAML front-matter for output paths and conditional generation |
| `_variables.toml` | Declares **per-invocation** extended top-level variables; read by agents and the validator |
| `_field_types.toml` | Declares allowed field type names |
| `_field_extra.toml` | Optional; declares valid keys for `fields[].extra` with type and `required_for` |
| `<bundle>/type_map.toml` | Optional; used with the `ty_map` helper for type mapping |
| `.crudignore` | Excludes templates from generation |

Path prefixes (e.g. `java/`, `vue/`, `resources/`) are mapped to host-project directories via `[paths.lang]` / `[paths.aux]` in `.crud/setup.toml`. See [README.md](../README.md#path-system).

## Front-matter

Any `.hbs` file may start with an optional YAML block wrapped in `---`:

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only          # never | force-only | always
generateWhen: has_import       # mutually exclusive with skipWhen
---
```

| Key | Description |
|-----|-------------|
| `basePath` / `base_path` | Output directory (relative to project root); may reference built-in or extended top-level variables |
| `filename` | Output file name (**single segment**, no `/`); may reference variables |
| `overwrite` | Overwrite policy; overrides global and user defaults |
| `generateWhen` / `generate_when` | Generate this file only when the condition is truthy; value is the **inner** expression of `{{#if ...}}` (no surrounding `{{ }}`) |
| `skipWhen` / `skip_when` | **Skip** this file when the condition is truthy; mutually exclusive with `generateWhen` |

Conditions follow Handlebars truthiness: `false`, missing, empty string, `0`, and empty arrays are falsy.

```yaml
---
generateWhen: has_import
filename: "{{model_pascal}}ImportDTO.java"
---
```

```yaml
---
skipWhen: "(eq mode \"slim\")"
filename: "{{model_pascal}}Service.java"
---
```

Files skipped by condition are marked `[skipped: condition]` in `gen` output. A mistyped variable evaluates falsy at gen time and the file is **silently skipped** — run `crud-cli validate` first.

---

## Built-in top-level variables

`crud-cli` **automatically injects** these into the render context on every `gen` run. Keys are always present; values change per invocation.

Do **not** declare keys with the same names in `_variables.toml` or `setup.toml` `[variables]` — that triggers `variable shadows built-in`.

### Entity and table

| Variable | Type | Description |
|----------|------|-------------|
| `model` | string | Entity/class name (raw value, e.g. `User`) |
| `model_pascal` | string | PascalCase, e.g. `User` |
| `model_snake` | string | snake_case, e.g. `user` |
| `model_camel` | string | camelCase, e.g. `user` |
| `model_kebab` | string | kebab-case, e.g. `user` |
| `table` | string | Primary table physical name |
| `table_comment` | string | Primary table description; `""` when omitted |
| `package` | string | Server-side package name (e.g. Java package) |
| `package_path` | string | `package` with `.` replaced by `/`, e.g. `com/acme/demo` |

### Primary key (derived from primary-table fields)

| Variable | Type | Description |
|----------|------|-------------|
| `pk_field` | string | Primary-key field name in camelCase |
| `pk_field_type` | string | Primary-key field raw type string |
| `pk_field_pascal` | string | Primary-key field name in PascalCase |

If no field in primary `fields` has `is_pk: true`, defaults are `id` / `Long` / `Id`.

### Master–detail (parent/child)

| Variable | Type | When no master–detail |
|----------|------|------------------------|
| `is_sub` | bool | `false` |
| `sub_table` | string | `""` |
| `sub_table_comment` | string | `""` |
| `sub_fields` | array | `[]` |
| `sub_model` | string | `""` |
| `sub_model_snake` | string | `""` |
| `sub_model_pascal` | string | `""` |
| `sub_model_camel` | string | `""` |
| `sub_model_kebab` | string | `""` |
| `sub_model_fk` | string | `""` |
| `sub_model_fk_pascal` | string | `""` |

When a master–detail relationship is present, `is_sub` is `true` and the rest are filled from the child entity and foreign-key column. `sub_model_fk` is the FK column in camelCase; `sub_model_fk_pascal` is often used for Java setter names.

### Field lists

| Variable | Type | Description |
|----------|------|-------------|
| `fields` | array | Primary-table field array |
| `sub_fields` | array | Child-table field array; `[]` when no master–detail |

Iterate in templates with `{{#each fields}}` / `{{#each sub_fields}}`; per-item properties are documented in the next section.

### Author and time

| Variable | Type | Description |
|----------|------|-------------|
| `git_user_name` | string | From git config |
| `git_user_email` | string | From git config |
| `user_name` | string | From `.crud/setup.user.toml`; falls back to `git_user_name` when empty |
| `user_email` | string | From `.crud/setup.user.toml`; falls back to `git_user_email` when empty |
| `date` | string | Local date, `YYYY-MM-DD` |
| `datetime` | string | Local date-time, `YYYY-MM-DD HH:MM:SS` |
| `year` | string | Four-digit year |

### Handlebars context specials

Inside `{{#each}}` blocks (provided by Handlebars; treated as valid by the validator):

| Variable | Description |
|----------|-------------|
| `this` | Current iteration item |
| `@index` | Zero-based index |
| `@key` | Key when iterating objects |
| `@first` / `@last` | Whether first / last item |
| `@root` | Root context |

## Field objects (`{{#each fields}}`)

Each item in `fields` / `sub_fields` exposes these **default** properties in templates:

| Property | Type | Description |
|----------|------|-------------|
| `name` | string | Column name (raw value) |
| `name_pascal` | string | PascalCase |
| `name_snake` | string | snake_case |
| `name_camel` | string | camelCase |
| `name_kebab` | string | kebab-case |
| `type` | string | Field type string (matches canonical name in `_field_types.toml`) |
| `is_pk` | bool | Primary key |
| `required` | bool | Required (`false` by default) |
| `comment` | string | Comment/label; `""` when omitted |
| `length` | number \| null | Length; `null` when omitted |
| `unique` | bool | Unique constraint; `false` when omitted |
| `default` | any \| null | Default value; `null` when omitted |

Example:

```handlebars
{{#each fields}}
  {{#if is_pk}}
    /** {{comment}} */
    private {{ty_map type}} {{name_camel}};
  {{/if}}
{{/each}}
```

With the `--fields` DSL, usually only `name`, `type`, and `is_pk` are populated, while `required` defaults to `false`; `comment` is `""`, `length` / `default` are `null`, and `unique` is `false`. For full metadata (comments, length, uniqueness, etc.), supply field details in the gen input.

### Extended field properties

Beyond the defaults above, **template bundles may define their own extensions**: extra key/value pairs from the caller are **flattened** into each field object and accessed at the same level as default properties inside `{{#each fields}}`.

Example: a bundle defines `query` (bool) and `dict_type` (string):

```handlebars
{{#each fields}}
  {{#if query}}
    <el-form-item label="{{comment}}">...</el-form-item>
  {{/if}}
{{/each}}
```

Semantics of extended keys are defined by the **template author** in `_field_extra.toml` (bundle root). Agents query them via `crud_describe_templates` (`field_extra` key in the response) rather than relying on inline documentation.

```toml
# _field_extra.toml
[options]
description  = "Enum option list; each item is {label, value}"
type         = "array"
required_for = ["enum", "radio"]

[dict_type]
description  = "Dictionary code binding for select/radio fields"
type         = "string"

[query]
description  = "Include this field as a query condition in list pages"
type         = "bool"
```

`_field_extra.toml` schema per key:

| Field | Required | Notes |
|---|---|---|
| `description` | yes | Human/agent-readable purpose |
| `type` | yes | `string` \| `number` \| `bool` \| `array` \| `object` |
| `required_for` | no | Field types that require this key (empty = always optional) |

`validate` static checks recognise default property names only; if an extended key triggers `unknown variable`, verify spelling against `_field_extra.toml`.

## Extended top-level variables

Besides built-in top-level variables, you may add **custom top-level variables** for front-matter, `{{#if}}`, and template bodies:

| Mechanism | Description |
|-----------|-------------|
| `_variables.toml` | Declare schema at bundle root (`type`, `default`, `required`, `description`); `description` is for agents |
| `.crud/setup.toml` → `[variables]` | Project-level defaults merged into top-level context |
| `--var key=value` | Per-invocation override |
| `variables` object in gen input | Per-invocation override; same as `--var` |

Priority: `--var` > gen input `variables` > schema `default`.

```toml
# _variables.toml example
[has_import]
description = "Whether to generate import button and importExcel endpoint"
type        = "bool"
default     = false

[module_name]
description = "Business module id for permission prefix and routes"
type        = "string"
required    = true
```

Custom top-level variables **must not** collide with built-ins or reserved names like `fields`. `validate` checks that referenced variables belong to: **built-ins** ∪ **schema declarations** ∪ **`[variables]` config**.

## Built-in helpers

`crud-cli` registers the helpers below on the Handlebars engine. Standard block helpers (`if`, `unless`, `each`, `with`, etc.) and subexpressions (e.g. `(eq a b)`) work as usual.

### Case conversion

Each accepts one string argument and returns the converted value:

| Helper | Example input | Output |
|--------|---------------|--------|
| `pascal_case` | `hello_world` | `HelloWorld` |
| `snake_case` | `HelloWorld` | `hello_world` |
| `camel_case` | `hello_world` | `helloWorld` |
| `kebab_case` | `hello_world` | `hello-world` |

```handlebars
{{pascal_case model_snake}}
{{camel_case "order_item_id"}}
```

### Brace wrapping (MyBatis / Vue placeholders)

The engine **does not HTML-escape** output, so Java generics like `<List<T>>` pass through unchanged. These helpers embed literal brace pairs in generated output without conflicting with Handlebars syntax:

| Helper | Example | Output |
|--------|---------|--------|
| `single_brace` | `{{single_brace name_camel}}` (context `name_camel=userId`) | `{userId}` |
| `double_brace` | `{{double_brace name_camel}}` | `{{userName}}` |

Common MyBatis patterns (`#` / `$` prefixes go **outside** the helper):

```handlebars
WHERE id = #{{single_brace pk_field}}
ORDER BY ${{single_brace pk_field}}
```

When a Vue template needs literal `{{variable}}` interpolation:

```handlebars
<span>{{double_brace name_camel}}</span>
```

### Type mapping `ty_map`

Maps neutral type names to target-stack types (e.g. Java `Integer`, TS `number`):

```handlebars
private {{ty_map type}} {{name_camel}};
```

The map comes from `type_map.toml` under the current bundle. On miss, behavior is governed by `.crud/setup.toml` → `[type_map].fallback`:

| fallback | Behavior |
|----------|----------|
| `passthrough` (default) | Emit the type string unchanged |
| `error` | Abort render |
| any other string | Replace with that fixed literal |

### Standard Handlebars (not registered separately)

Provided by Handlebars; `validate` will not report `missing_helper`:

- **Block helpers:** `{{#if}}` / `{{#unless}}` / `{{#each}}` / `{{#with}}`
- **Subexpressions:** `(eq a b)`, `(ne a b)`, `(and a b)`, `(or a b)`, `(not x)`, etc., often used in front-matter conditions
- **Paths:** `../` for parent context; `lookup` for dynamic property access

## What agents should read from the target project

Templates should make generated code **byte-identical** to the host project's style. Before creating or adapting a bundle, agents should study the target repo systematically rather than copying generic scaffolds. The checklist below is grouped by stack — use only what applies.

### All projects

| Focus | What to determine |
|-------|-------------------|
| Directory layout | Roots for source, resources, tests, docs, frontend; monorepo / multi-module structure |
| `.crud/setup.toml` | Whether `[project]`, `[paths.lang]`, `[paths.aux]`, `[type_map]`, `[variables]` are configured |
| Existing CRUD samples | One or two **hand-written** files of the same kind as the feature being generated — the template "gold standard" |
| Naming conventions | Case and prefixes for classes/files/tables/columns/API paths (e.g. `XxxController`, `sys_` table prefix) |
| Comments and file headers | Whether `@author`, copyright blocks, or generation dates are required (built-ins `user_name`, `date` help) |
| Auth and security | Annotations, middleware, route guards — naming and placement |
| Errors and responses | Unified response wrappers, error codes, pagination shape |
| Logging and audit | Logger in use; whether operation logs need templating |
| Tests | Test directory, base classes, mocking style |

### Java / Kotlin (Spring, MyBatis, JPA, etc.)

| Focus | What to determine |
|-------|-------------------|
| Package layout | Layering and naming for `controller` / `service` / `mapper` / `domain` / `dto` / `vo` |
| Web layer | `@RestController` path prefix, HTTP verbs, parameter annotations (`@RequestBody`, `@PathVariable`) |
| Unified responses | Wrapper types (`R`, `AjaxResult`, `Result`) and static factory method names |
| Exceptions | Business exception base class, global `@ControllerAdvice`, error-code enums |
| Validation | `javax` / `jakarta.validation` annotation habits, `@Validated` groups |
| Persistence | MyBatis XML vs annotations; `#{}` / `${}` habits; PK strategy and logical-delete fields |
| ORM entities | Base classes (`BaseEntity`), Lombok combinations, field mapping, `@TableLogic` |
| Pagination | `PageHelper`, `IPage`, request/response DTO field names |
| Import/export | Excel module DTOs and controller method signatures if present |
| Transactions and permissions | `@Transactional` placement; `@PreAuthorize` / custom permission string format |

### TypeScript / JavaScript

| Focus | What to determine |
|-------|-------------------|
| Runtime framework | NestJS modules/DTOs/decorators, or Express/Fastify routes and middleware |
| Validation | `class-validator`, Zod, Joi, and how errors are thrown |
| ORM | Prisma schema naming, TypeORM entity decorators, Sequelize models |
| API layer | Controller/handler return types, interceptors, exception filters |
| Frontend (same repo) | See Vue/React below |

### Go

| Focus | What to determine |
|-------|-------------------|
| Package paths | Feature-based vs layer-based layout under `internal/` |
| Web framework | Gin/Echo/Fiber route registration, handler signatures, middleware chain |
| Error handling | Custom `error` types, HTTP status mapping |
| Data access | GORM / sqlx tags, repository interface locations |
| Config and DI | Whether wire/fx affects generated file structure |

### Python

| Focus | What to determine |
|-------|-------------------|
| Framework | Django apps/models/admin/serializers, or FastAPI routers/dependencies |
| Models | Pydantic `BaseModel`, SQLAlchemy declarative models, Alembic migration habits |
| Validation and responses | `HTTPException`, unified `response_model`, pagination schemas |
| Async | Whether `async def` is standard; session lifecycle |

### C# / .NET

| Focus | What to determine |
|-------|-------------------|
| Project type | Web API, Minimal API, Clean Architecture layer directories |
| Data annotations | FluentValidation vs DataAnnotations |
| EF Core | DbContext, entity configuration, migration commands |
| Unified results | `ActionResult<T>`, ProblemDetails, custom `ApiResponse` |

### PHP

| Focus | What to determine |
|-------|-------------------|
| Framework | Laravel Controller/Request/Resource/Policy conventions, or Symfony bundle layout |
| ORM | Eloquent model traits, migration file naming |
| Validation | FormRequest, rule array style |

### Rust

| Focus | What to determine |
|-------|-------------------|
| Web | axum / actix-web routes and extractors |
| Data | sqlx / diesel models and migration directories |
| Errors | `thiserror`, `anyhow`, IntoResponse mapping |

### Frontend (Vue / React)

| Focus | What to determine |
|-------|-------------------|
| Directories | Actual paths for views/pages, components, api, router, store |
| API client | axios wrapper, request/response types, baseURL |
| List pages | Table component, search form, pagination param names, permission directives (`v-hasPermi`, etc.) |
| Forms and validation | UI library (Element Plus, Ant Design) field binding and rules |
| Routing | Dynamic routes, meta fields, lazy-load pattern |
| State | Whether Pinia/Vuex/Redux participates in CRUD pages |

### Database and SQL templates

| Focus | What to determine |
|-------|-------------------|
| Dialect | MySQL / PostgreSQL types and index syntax |
| Naming | Table prefixes, column naming, charset and engine defaults |
| Separate DDL | Whether DDL templates live in a separate bundle when using `--stdout --type sql` |

### Recommended agent workflow

1. Read `.crud/setup.toml` and the bundle's `_variables.toml`, `_field_types.toml`, `_field_extra.toml`.
2. Locate **existing implementations** of the same feature in the target project (controller + service + frontend list page, as applicable).
3. List differences from generic scaffolds (return types, base classes, annotations, path prefixes, permission strings).
4. Write or edit `.hbs` files; align output paths via front-matter; declare extended top-level variables in `_variables.toml`; declare extended field keys in `_field_extra.toml`.
5. Run `crud-cli validate`, then compare output with `--dry-run` / `--stdout` against the gold standard.
6. Close gaps by **changing templates**, not by hand-editing generated code afterward.

## Engine behavior

- **No HTML escaping:** `{{type}}` and similar emit literally; `<List<T>>` is preserved.
- **Deterministic validation:** `validate` statically analyzes variable references; variables in conditional front-matter must appear in the schema or built-in list.
- **Transactional writes:** A conflict on any output file can roll back the entire batch (depending on overwrite policy).

## Common errors

| Message | What to check |
|---------|---------------|
| `unknown variable` / `UnknownVariable` | Template references a variable not in built-ins, `_variables.toml`, or `[variables]` |
| `variable shadows built-in` | `_variables.toml` or `[variables]` uses a built-in name |
| `missing_helper` | Misspelled helper; confirm helper is listed above or is a Handlebars built-in |
| `helper not found` (render stage) | Same as above; or `ty_map` with fallback=error on an unmapped type |
| File marked `[skipped: condition]` | `generateWhen` / `skipWhen` evaluated falsy; check values and spelling |
| Condition silently skips file | Undeclared variables are falsy — run `validate` first |
| `invalid filename` | front-matter `filename` contains `/` or path-traversal segments |
| front-matter YAML parse failure | Quote values that contain `{{` |
