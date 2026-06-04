# crud-cli

**Languages:** English · [简体中文](./README.zh.md)

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

A Rust CLI that pairs with AI coding agents (Claude Code, Cline, Copilot, …) to generate admin/CRUD scaffolding.

The dominant cost of an AI agent is **output tokens** — every time an agent writes out full template files, you pay in tokens and wait time. `crud-cli` keeps templates local: the agent emits a short command plus structured data, and the CLI renders everything on your machine. This sharply cuts output cost and speeds up generation, while keeping results byte-identical to your project's house style.

## Token cost reference

The table below uses [tiktoken](https://github.com/openai/tiktoken) to give a rough sense of scale, comparing the token count of code produced by `crud-cli gen` (native) against the structured command needed to trigger it (json). Data comes from `_example_sub.json` and `_example_tree.json` in the [crud-templates](https://github.com/ttTennessee/crud-templates) repository.

| Scenario | Generated code | Input command | Difference |
|----------|--------------|--------------|------------|
| Master-detail (sub) | 18,325 | 1,151 | ~94% |
| Tree structure | 18,166 | 689 | ~96% |

> **Note:** These numbers show the size difference between generated output and the input command — not the actual token savings. Real usage also includes system prompt, conversation context, and MCP tool call overhead. Treat these as rough order-of-magnitude figures only.

**Is this a good fit?**

- If your project is mostly **standard CRUD pages** (list, form, detail, import/export), this tool pays off well — repetitive template code is exactly where agents waste the most tokens.
- If your business logic is complex and each table needs heavy custom code, template reuse is low and the benefit may not justify adopting this tool.

## Install

Prebuilt binaries from [Releases](https://github.com/ttTennessee/crud-cli/releases) (include the MCP server):

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

Or build from source (recent stable Rust toolchain; no committed MSRV):

```bash
cargo build --release --features full   # CLI + MCP server
```

Full install reference: [docs/quickstart.md](docs/quickstart.md#install).

## Hello world

```bash
# 1. Initialize project config
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views

# 2. Install a ready-made template bundle (or write your own under .crud/templates/)
crud-cli template install

# 3. Generate
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

End-to-end walkthrough with explanations: [docs/quickstart.md](docs/quickstart.md).

## Default template repository

[crud-templates](https://github.com/ttTennessee/crud-templates) is the companion template repository, hosting ready-to-use CRUD bundles installable via `crud-cli template install`. The admin-framework landscape is vast and one maintainer can't cover it alone — contributions of new template bundles are very welcome.

## Documentation

| Topic | Where |
|---|---|
| Install, basic usage, CLI subcommand reference | [docs/quickstart.md](docs/quickstart.md) |
| Template structure and authoring guide | [docs/templates.md](docs/templates.md) |
| MCP server — configuration, tools, resources | [docs/mcp.md](docs/mcp.md) |
| `entity.json` schema reference | [docs/entity.md](docs/entity.md) |
| Template authoring spec served by MCP | [agent-resources/template-authoring.md](agent-resources/template-authoring.md) |
| Contributor docs | [docs/dev/](docs/dev/) |

## Architecture

Single crate with three module layers; the upper two are gated by Cargo features so consumers only pay for what they use.

- `src/core/` — pure logic: config, paths, template engine, transactional writer, validator, typed errors. No clap, no inquire, no tokio.
- `src/cli/` — feature `cli` (default). `clap` surface, `inquire` wizard, agent-mode JSON, human output.
- `src/mcp/` — feature `mcp`. MCP server (`crud-cli mcp`) on `rmcp` + `tokio`, reusing `core` over stdio.

Features: `default = ["cli"]`, `cli`, `mcp`, `full = ["cli", "mcp"]`. The library `crud_cli` can be consumed with `--no-default-features` for embedding.

## License

MIT — see [LICENSE](./LICENSE). Copyright (c) 2026 Yujie Jin.
