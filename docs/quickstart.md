# Quickstart

**Languages:** English · [简体中文](zh-CN/quickstart.md)

This guide covers installation and the everyday CLI workflow. For the MCP server (driven by AI agents), see [mcp.md](mcp.md).

## Install

### Prebuilt binaries (recommended)

Grab the archive for your platform from [Releases](https://github.com/ttTennessee/crud-cli/releases), extract it, and put `crud-cli` (or `crud-cli.exe`) anywhere on your `PATH`.

One-liners:

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

Prebuilt binaries include the MCP server (`crud-cli mcp`).

### Build from source

Requires a recent stable Rust toolchain (no committed MSRV).

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release                 # CLI only
cargo build --release --features full # CLI + MCP server
```

The binary lands at `./target/release/crud-cli`.

## End-to-end walkthrough

A fresh project, from zero to generated files:

### 1. Initialize project config

`crud-cli setup` writes `.crud/setup.toml` (shared, checked in) and optionally `.crud/setup.user.toml` (per-developer, gitignored).

Non-interactive:

```bash
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views \
  --aux resources=src/main/resources --aux doc=doc/api
```

Interactive (drop `--project` and the flags to launch the wizard):

```bash
crud-cli setup
```

Per-developer overrides (name appears in generated file headers; overwrite policy controls when `gen` is allowed to clobber existing files):

```bash
crud-cli setup --user-name "Alice" --user-email alice@example.com \
  --overwrite-policy force-only --enabled-types backend
```

### 2. Add a template

Drop a `.hbs` file under `.crud/templates/`. The first path segment (here `java/`) is a **prefix** resolved against `[paths.lang]` in `setup.toml`.

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

See the [main README](../README.md) for the full template authoring reference (front-matter, helpers, `_variables.toml`, etc.). A dedicated [templates.md](templates.md) is planned.

### 3. Validate

Run before generation (and before committing template changes):

```bash
crud-cli validate
# validate ok: 1 templates
```

Catches: Handlebars syntax errors, references to undeclared variables, unsafe `filename` / `basePath`, broken YAML front-matter.

### 4. Generate

Field DSL — concise, good for quick scaffolds:

```bash
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

Output lands at `<paths.lang.java>/com/acme/demo/controller/UserController.java`.

JSON entity input — for rich field metadata (comments, length, unique, defaults, master–detail, per-field extras):

```bash
crud-cli gen --file user.json
```

See [entity.md](entity.md) for the `entity.json` schema.

## CLI command reference

Each subcommand below is summarized with its typical use. For the complete flag list, run `crud-cli <subcommand> --help` — that's the authoritative source and stays in sync with the binary.

### `crud-cli setup`

Interactive wizard or non-interactive flag mode. Writes `.crud/setup.toml` (shared) and/or `.crud/setup.user.toml` (per-developer). Run once per project, plus once per developer if user-level config is needed.

### `crud-cli gen <Name>`

Render templates into the project tree.

- **Field DSL** (`--fields "name:type,..."`) for quick scaffolds.
- **JSON file** (`--file entity.json`) for rich field metadata or master–detail.
- **Per-call variables** via repeatable `--var key=value` or the JSON `variables` object — values declared in `_variables.toml`.
- **`--dry-run`** lists what would be written without touching disk.
- **`--stdout`** prints rendered output to stdout instead of writing files. With `--type sql`, useful for letting an agent show DDL for user confirmation before the real generation.
- **Transactional**: on any conflict, the entire batch rolls back — nothing partial lands on disk.

### `crud-cli validate`

Pre-flight check over `.crud/templates/`: Handlebars syntax, undeclared variable references, YAML front-matter, `filename` / `basePath` safety, fixture render. Run before commit and in CI.

### `crud-cli template install [<name>[@<version>]]`

Download a template bundle from a GitHub repo into `~/.crud/templates/<name>/<version>/`. Interactive name/version pickers (versions are labelled with installed / locally-modified / repo-updated status). Default repo is configurable in `~/.crud/config.toml` under `[templates].repo`.

### `crud-cli template list`

List installed template bundles in `~/.crud/templates/`.

### `crud-cli template use <name>[@<version>]`

Point the current project's `[project].template` at an installed bundle (syncs backend / frontend selection).

### `crud-cli mcp`

Start the embedded MCP server over stdio for AI agents. Requires a binary built with `--features full` (prebuilt binaries qualify). See [mcp.md](mcp.md) for client configuration and tool reference.

## Agent mode (`--agent`)

A global flag that switches CLI output to a machine-readable shape — useful when an agent shells out to `crud-cli` directly (without using the MCP server):

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- **Success**: exit 0, empty stdout.
- **Failure**: exit non-zero, single JSON object on stderr with `code`, `message`, `flag`, `value`, `remediation`, plus context-specific `details`. Safe to feed back to the model.

For most agent integrations the MCP server is preferable — it offers structured tool calls and avoids parsing CLI output. Use `--agent` mode for scripts and CI.
