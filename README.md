# crud-cli

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

A Rust CLI that pairs with AI coding agents (Claude Code, Cline, Copilot, …)
to generate admin/CRUD scaffolding **without** burning the agent's tokens on
template boilerplate.

The agent emits a short command plus structured data; `crud-cli` renders the
templates locally. In practice this drops per-CRUD token cost from ~2000+ to
~50 — roughly a 40× reduction — while keeping output byte-identical to your
project's house style.

[中文文档](./README.zh.md)

## Status

Implemented:

- `crud-cli setup` — interactive wizard and non-interactive flag mode. Writes
  the shared project config `.crud/setup.toml` or the per-developer
  `.crud/setup.user.toml`.
- `crud-cli gen` — render templates with field DSL or JSON entity file. Per-call
  variable injection via repeatable `--var key=value` or JSON `variables`.
- `crud-cli validate` — pre-flight check: handlebars syntax, unknown variables,
  YAML front-matter, `filename`/`basePath` safety, fixture render.
- Front-matter `basePath` / `filename` / `overwrite` with framework-prefix
  rebasing across `java/`, `resources/`, `doc/`, `vue/`, `react/`, `nest/`.
- `_variables.toml` schema for declaring per-call switches (with type, default,
  required, natural-language description for agent consumption).
- Transactional two-phase write — on conflict, nothing lands on disk.
- Agent mode (`--agent`): structured JSON errors on stderr, empty stdout on
  success.

Not yet implemented: `template install`, `template list`.

## Install

Requires Rust ≥ 1.75.

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release
# binary at ./target/release/crud-cli
```

A `cargo install` / prebuilt-binary path will land with the v0.1 release.

## Quick start

### 1. Setup

Project config (checked in, shared by all developers):

```bash
crud-cli setup --project --backend spring-boot --frontend vue \
  --component-library element-plus
```

User config (per developer, gitignored):

```bash
crud-cli setup --user-name "Alice" --user-email alice@example.com \
  --overwrite-policy force-only --enabled-types backend
```

Drop `--project` / the user flags to launch the interactive wizard.

### 2. Drop in a template

`.crud/templates/java/Controller.java.hbs`:

```handlebars
---
basePath: "java/{{package_path}}/controller"
filename: "{{model_pascal}}Controller.java"
---
package {{package}}.controller;

@RestController
@RequestMapping("/{{model_kebab}}")
public class {{model_pascal}}Controller {
    // ...
}
```

### 3. Generate

```bash
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

Output lands at `<java_base>/com/acme/demo/controller/UserController.java`.

### 4. Validate before commit

```bash
crud-cli validate
# validate ok: 1 templates
```

## Path system

Templates are project-agnostic; per-project layout is driven by
`.crud/setup.toml [paths]`. Each prefix in the template path is rebased to the
configured base:

| Template prefix | Config key | SpringBoot default | Vue default |
|---|---|---|---|
| `java/` | `paths.java_base` | `src/main/java` | — |
| `resources/` | `paths.resources_base` | `src/main/resources` | — |
| `doc/` | `paths.doc_base` | `doc/api` | — |
| `vue/` | `paths.vue_base` | — | `src/views` |
| `react/` | `paths.react_base` | — | `src/views` |
| `nest/` | `paths.nest_base` | `src` (Nest backend) | — |

Override in `setup.toml` to match your monorepo layout:

```toml
[paths]
java_base = "backend/api/src/main/java"
resources_base = "backend/api/src/main/resources"
doc_base = "docs/api"
```

A template at `.crud/templates/java/Foo.hbs` (or with
`basePath: "java/{{package_path}}/foo"`) lands under the configured
`java_base` regardless of how the host project is laid out.

## Template authoring

### Front-matter

Optional YAML block at the top of any `.hbs` file:

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only          # never | force-only | always
---
```

`basePath` may reference any built-in or schema-declared variable. `filename`
must be a single path segment (no `/`).

### Built-in context

Always available in templates:

- `{{model}}`, `{{model_pascal}}`, `{{model_snake}}`, `{{model_camel}}`,
  `{{model_kebab}}`
- `{{table}}`, `{{package}}`, `{{package_path}}` (dots → slashes)
- `{{fields}}` — iterate with `{{#each fields}}`; each item exposes `name`,
  `name_pascal`, `name_snake`, `name_camel`, `name_kebab`, `type`, `is_pk`,
  `nullable`
- `{{git_user_name}}`, `{{git_user_email}}`, `{{user_name}}`, `{{user_email}}`
- `{{date}}`, `{{datetime}}`, `{{year}}`

Helpers: `pascal_case`, `snake_case`, `camel_case`, `kebab_case` (e.g.
`{{pascal_case "hello_world"}}` → `HelloWorld`).

### Per-call variables (`_variables.toml`)

Declare variables a template family expects at `.crud/templates/_variables.toml`:

```toml
[has_import]
description = "Generate import button + importExcel endpoint"
type        = "bool"          # bool | string | number
default     = false

[has_export]
description = "Generate export endpoint"
type        = "bool"
default     = false

[table_comment]
description = "Business caption for Swagger annotations and class docs"
type        = "string"
required    = true
```

Pass values at gen time:

```bash
crud-cli gen User --fields "..." --package ... --table ... \
  --var has_import=true --var table_comment="System User"
```

Priority: `--var` > JSON `variables` > schema `default`. Missing `required` →
error. Undeclared key (not in `_variables.toml`) → error.

The `description` field is the contract agents read to understand what to fill.

### JSON entity input

For rich field metadata, use `--file`:

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true },
    { "name": "email", "type": "String", "extra": { "unique": true } }
  ],
  "variables": {
    "has_import": true,
    "table_comment": "System User"
  }
}
```

```bash
crud-cli gen --file user.json
```

CLI flags (`--name`, `--package`, `--table`, `--var`) override JSON values.

## Configuration files

| File | Scope | Tracked | Contents |
|---|---|---|---|
| `.crud/setup.toml` | Project | Yes | `[project]`, `[paths]`, `[variables]`, `[templates.outputs]` |
| `.crud/setup.user.toml` | Developer | No | `[user]`, `[overwrite]`, `[scope]` |
| `.crud/templates/_variables.toml` | Project | Yes | Per-call variable schema |
| `.crud/templates/**/*.hbs` | Project | Yes | Templates |
| `.crud/templates/.crudignore` | Project | Yes | Exclude files from discovery |

All TOML schemas use `deny_unknown_fields` — typos and drift surface
immediately rather than silently changing behavior.

## Agent mode

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- Success: exit 0, empty stdout.
- Failure: exit non-zero, single JSON object on stderr (`code`, `message`,
  `flag`, `value`, `remediation`, plus context-specific `details`). Safe to
  feed back to the model.

## Architecture

Two layers, strictly separated so a future MCP server can reuse the core:

- `src/core/` — pure logic: config parsing, path resolution, template engine,
  transactional filesystem writer, validator, variable schema, typed
  `thiserror` errors. No clap, no inquire, no I/O concerns beyond what's needed.
- `src/cli/` — `clap` surface, `inquire` wizard, agent-mode JSON output,
  human-readable output. Depends on `core`; `core` never depends back.

The `cli` feature gate (`--no-default-features`) lets you depend on `crud_cli`
as a library without pulling clap/inquire.

## Testing

```bash
cargo test            # unit + integration + contract tests
```

Contract tests (`tests/contracts/`) lock the agent-facing surface: panic
behavior, JSON error envelope shape, byte-identical setup output.

## License

MIT OR Apache-2.0
