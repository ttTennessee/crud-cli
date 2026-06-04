# crud-cli

**Languages:** English · [简体中文](./README.zh.md)

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

A Rust CLI that pairs with AI coding agents (Claude Code, Cline, Copilot, …)
to generate admin/CRUD scaffolding.

The dominant cost of an AI agent is **output tokens** — every time an agent
writes out full template files, you pay in tokens and wait time. `crud-cli`
keeps templates local: the agent emits a short command plus structured data,
and the CLI renders everything on your machine. This sharply cuts output cost
and speeds up generation, while keeping results byte-identical to your
project's house style.

## Token cost reference

The table below uses [tiktoken](https://github.com/openai/tiktoken) to give a rough sense of scale,
comparing the token count of code produced by `crud-cli gen` (native) against the structured
command needed to trigger it (json).
Data comes from `_example_sub.json` and `_example_tree.json` in the
[crud-templates](https://github.com/ttTennessee/crud-templates) repository.

| Scenario | Generated code | Input command | Difference |
|----------|--------------|--------------|------------|
| Master-detail (sub) | 18,325 | 1,151 | ~94% |
| Tree structure | 18,166 | 689 | ~96% |

> **Note:** These numbers show the size difference between generated output and the input command —
> not the actual token savings. Real usage also includes system prompt, conversation context, and
> MCP tool call overhead. Treat these as rough order-of-magnitude figures only.

**Is this a good fit?**

- If your project is mostly **standard CRUD pages** (list, form, detail,
  import/export), this tool pays off well — repetitive template code is exactly
  where agents waste the most tokens.
- If your business logic is complex and each table needs heavy custom code,
  template reuse is low and the benefit may not justify adopting this tool.

## Default template repository

[crud-templates](https://github.com/ttTennessee/crud-templates) is the companion template repository,
providing ready-to-use Java + Vue CRUD templates installable via `crud-cli template install`.

## Install

### Prebuilt binaries (recommended)

Download the archive for your platform from [Releases](https://github.com/ttTennessee/crud-cli/releases),
extract it, and place `crud-cli` (or `crud-cli.exe` on Windows) anywhere on your PATH.

Or use the one-line installer:

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

### Build from source

Requires a recent stable Rust toolchain (no committed MSRV).

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release                 # CLI only
cargo build --release --features full # CLI + MCP server
# binary at ./target/release/crud-cli
```

## Quick start

### 1. Setup

Project config (checked in, shared by all developers). `--backend` /
`--frontend` take a language identifier; `--lang` / `--aux` set the path map:

```bash
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views \
  --aux resources=src/main/resources --aux doc=doc/api
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

Output lands at `<paths.lang.java>/com/acme/demo/controller/UserController.java`.

### 4. Validate before commit

```bash
crud-cli validate
# validate ok: 1 templates
```

## Path system

Templates are project-agnostic; per-project layout is driven by the two path
maps in `.crud/setup.toml`. The first path segment of a template's location
(its **prefix**) is looked up in `[paths.lang]` first, then `[paths.aux]`, and
the prefix is rebased to the configured directory. The model is language-based
and open-ended — there is no fixed list of framework prefixes; any key you add
to `[paths.lang]` / `[paths.aux]` becomes a usable prefix.

Conventional defaults seeded by `setup` (depend on the chosen languages):

| Prefix | Map | Default | Seeded for |
|---|---|---|---|
| `java` | `[paths.lang]` | `src/main/java` | backend = java |
| `ts` | `[paths.lang]` | `src` | backend = typescript |
| `go` | `[paths.lang]` | `internal` | backend = go |
| `python` | `[paths.lang]` | `src` | backend = python |
| `vue` | `[paths.lang]` | `src/views` | frontend = vue |
| `react` | `[paths.lang]` | `src/views` | frontend = react |
| `resources` | `[paths.aux]` | `src/main/resources` | backend = java |
| `doc` | `[paths.aux]` | `doc/api` | most backends |

Override in `setup.toml` to match your monorepo layout:

```toml
[paths.lang]
java = "backend/api/src/main/java"
vue = "frontend/src/views"

[paths.aux]
resources = "backend/api/src/main/resources"
doc = "docs/api"
```

A template at `.crud/templates/java/Foo.hbs` (or with
`basePath: "java/{{package_path}}/foo"`) lands under the configured `java`
path regardless of how the host project is laid out.

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

**Conditional rendering** — `generateWhen` / `skipWhen` gate whether the file is
generated at all (mutually exclusive; setting both errors). The value is the
condition part of an `{{#if ...}}` (no surrounding `{{ }}`), evaluated with
Handlebars truthiness: `false`, missing, empty string, `0`, and empty arrays all
count as false. Pair it with a `_variables.toml` toggle to emit a file only when
needed:

```yaml
---
generateWhen: has_import          # generate only when has_import is truthy
filename: "{{model_pascal}}ImportDTO.java"
---
```

```yaml
---
skipWhen: is_readonly             # inverse of generateWhen: skip when truthy
filename: "{{model_pascal}}Service.java"
---
```

Condition-skipped files are reported separately as `[skipped: condition]`,
distinct from "skipped because it exists". `validate` checks that variables
referenced in a condition are declared — a typo'd variable evaluates falsy and
**silently skips** the file at gen time, so validate first.

### Built-in context

Always available in templates:

- `{{model}}`, `{{model_pascal}}`, `{{model_snake}}`, `{{model_camel}}`,
  `{{model_kebab}}`
- `{{table}}`, `{{table_comment}}` (optional entity/table business caption;
  `--table-comment`, JSON `table_comment`, or empty), `{{package}}`,
  `{{package_path}}` (dots → slashes)
- `{{fields}}` — iterate with `{{#each fields}}`; each item exposes `name`,
  `name_pascal`, `name_snake`, `name_camel`, `name_kebab`, `type`, `is_pk`,
  `required`, `comment`, `length`, `unique`, `default`. The last four come from
  the JSON `--file` FieldSpec (see below); the `--fields` DSL omits this
  metadata, so `comment` is empty, `length`/`default` are `null`, and `unique`
  is `false`. A DDL template emitting `CREATE TABLE` consumes exactly these.
- `{{git_user_name}}`, `{{git_user_email}}`, `{{user_name}}`, `{{user_email}}`
- `{{date}}`, `{{datetime}}`, `{{year}}`

Helpers: `pascal_case`, `snake_case`, `camel_case`, `kebab_case` (e.g.
`{{pascal_case "hello_world"}}` → `HelloWorld`); `single_brace`, `double_brace`
(one/two brace layers for MyBatis and Vue placeholders):

- `{{single_brace name_camel}}` → `{userId}`; prefix in template: `#{{single_brace …}}` → `#{…}`,
  `${{single_brace …}}` → `${…}`.
- `{{double_brace name_camel}}` → `{{userName}}` (Vue interpolation).

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

Full schema reference: [agent-resources/json-entity-input.md](agent-resources/json-entity-input.md) (written as a terse spec for LLM agents — same source of truth the MCP server serves).

For rich field metadata, use `--file`. Each field (FieldSpec) accepts `name`,
`type`, `is_pk`, `required`, `length`, `unique`, `default`, `comment`, and a
free-form `extra` map; all of them surface in the `{{#each fields}}` context.

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "primary key" },
    { "name": "email", "type": "String", "length": 128, "unique": true, "comment": "login email" }
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
| `.crud/setup.toml` | Project | Yes | `[project]`, `[paths.lang]`, `[paths.aux]`, `[variables]`, `[templates.outputs]`, `[type_map]` |
| `.crud/setup.user.toml` | Developer | No | `[user]`, `[overwrite]`, `[scope]` |
| `.crud/templates/_variables.toml` | Project | Yes | Per-call variable schema |
| `.crud/templates/**/*.hbs` | Project | Yes | Templates |
| `.crud/templates/.crudignore` | Project | Yes | Exclude files from discovery |
| `~/.crud/config.toml` | Global | No | `[templates].repo` — default GitHub repo for `template install` |
| `~/.crud/templates/<name>/<version>/` | Global | No | Installed template bundles |

All TOML schemas use `deny_unknown_fields` — typos and drift surface
immediately rather than silently changing behavior.

## MCP server

`crud-cli` ships an embedded MCP server so AI agents can drive code generation through tool calls instead of shell invocations.

```bash
cargo build --release --features full   # or install a prebuilt binary
crud-cli mcp
```

Configure your MCP client with `command: "crud-cli"`, `args: ["mcp", "--path", "/abs/path/to/project"]`. The server exposes tools (`crud_describe_templates`, `crud_preview`, `crud_generate`, …), resources (`crud://schema/entity`, `crud://templates/variables`, …), and a `crud_template_authoring` prompt sourced from [`agent-resources/`](agent-resources/).

Full reference: [docs/mcp-server.md](docs/mcp-server.md).

## Agent mode

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- Success: exit 0, empty stdout.
- Failure: exit non-zero, single JSON object on stderr (`code`, `message`,
  `flag`, `value`, `remediation`, plus context-specific `details`). Safe to
  feed back to the model.

## Architecture

Single crate with three module layers; the upper two are gated by Cargo features so consumers only pay for what they use.

- `src/core/` — pure logic: config parsing, path resolution, template engine, transactional filesystem writer, validator, variable schema, typed `thiserror` errors. No clap, no inquire, no tokio.
- `src/cli/` — feature `cli` (default). `clap` surface, `inquire` wizard, agent-mode JSON output, human-readable output. Depends on `core`.
- `src/mcp/` — feature `mcp`. MCP server (`crud-cli mcp`) built on `rmcp` + `tokio`, serving the same `core` APIs to LLM agents over stdio. Exposes machine-readable specs from [`agent-resources/`](agent-resources/) as MCP prompts and resources. Depends on `core` only — never on `cli`.

Features: `default = ["cli"]`, `cli`, `mcp`, `full = ["cli", "mcp"]`. The library `crud_cli` can be consumed with `--no-default-features` for embedding without clap/inquire/tokio.

## Testing

```bash
cargo test            # unit + integration + contract tests
```

Contract tests (`tests/contracts/`) lock the agent-facing surface: panic
behavior, JSON error envelope shape, byte-identical setup output.

## License

MIT — see [LICENSE](./LICENSE). Copyright (c) 2026 Yujie Jin.
