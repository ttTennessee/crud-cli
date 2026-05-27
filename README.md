# crud-cli

A Rust CLI that pairs with AI coding agents (Claude Code, Cline, Copilot, …) to
generate admin/CRUD scaffolding **without** burning the agent's tokens on
template boilerplate.

The agent emits a short command plus structured data; `crud-cli` renders the
templates locally. In practice this drops per-CRUD token cost from ~2000+ to
~50 — roughly a 40× reduction — while keeping output byte-identical to your
project's house style.

> 中文版: [README.zh.md](./README.zh.md)

## Status

Early development. The following is implemented today:

- `crud-cli setup` — interactive wizard **and** non-interactive flag mode for
  writing `.crud/setup.toml` (backend / frontend / component-library /
  overwrite-policy).
- Transactional two-phase plan + commit filesystem writer (no partial writes
  on failure).
- Agent mode (`--agent`): structured JSON error envelopes on stderr, empty
  stdout on success — designed to be parsed by an LLM.
- Handlebars template engine wiring (rendering pipeline; user-facing `gen`
  command lands in a later phase).

Not yet implemented: `gen`, `template install`, `validate`, `template list`.
See the PRD (`prd.html`) for the full target surface.

## Install

Requires Rust ≥ 1.75.

```bash
git clone <this repo>
cd crud-cli
cargo build --release
# binary at ./target/release/crud-cli
```

A `cargo install` / prebuilt-binary path will land with the v0.1 release.

## Quick start

### Interactive setup

```bash
crud-cli setup
```

Walks you through four choices and writes `.crud/setup.toml` in the current
project root.

### Non-interactive setup (for agents / CI)

All four flags are required when any flag is passed:

```bash
crud-cli setup \
  --backend spring-boot \
  --frontend vue \
  --component-library element-plus \
  --overwrite-policy never
```

Flag values:

| Flag | Values |
|---|---|
| `--backend` | `spring-boot`, `nest`, `none` |
| `--frontend` | `vue`, `react`, `none` |
| `--component-library` | `element-plus`, `antd`, `naive-ui`, `none` |
| `--overwrite-policy` | `never`, `force-only`, `always` |
| `--force` | only honored when `overwrite-policy=force-only` |

### Agent mode

```bash
crud-cli --agent setup --backend nest --frontend react \
  --component-library antd --overwrite-policy never
```

- Success: exit 0, empty stdout.
- Failure: exit non-zero, single JSON object on stderr with `code`, `message`,
  `flag`, `value`, `remediation`. Safe to feed back to the model.

## Configuration

Project config lives at `<project-root>/.crud/setup.toml`. Templates are
resolved from:

1. `<project-root>/.crud/templates/` (project-local, wins)
2. `~/.crud/templates/<name>/` (user-global)

The TOML schema is locked — unknown keys are rejected so an agent can't drift
the file shape.

## Architecture

Two layers, strictly separated so a future MCP server can reuse the core:

- `src/core/` — pure logic: config parsing, path resolution, template engine,
  transactional filesystem writer, typed `thiserror` errors. No clap, no
  inquire, no I/O concerns beyond what's needed.
- `src/cli/` — `clap` surface, `inquire` wizard, agent-mode JSON output,
  human-readable output. Depends on `core`; `core` never depends back.

The `cli` feature gate (`--no-default-features`) lets you depend on
`crud_cli` as a library without pulling clap/inquire.

## Testing

```bash
cargo test            # unit + integration + contract tests
```

Contract tests (`tests/contracts/`) lock the agent-facing surface: panic
behavior, JSON error envelope shape, and byte-identical setup output.

## License

MIT OR Apache-2.0
