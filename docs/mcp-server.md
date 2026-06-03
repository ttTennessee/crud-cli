# MCP Server (`crud-cli mcp`)

> **Other languages:** [简体中文](zh-CN/mcp-server.md)

Exposes `crud-cli` code generation to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io/). Single entry point: **`crud-cli mcp`**.

## Build

```bash
cargo build --release --features full
crud-cli mcp
```

Configure your MCP client with `command`: `crud-cli` and `args`: `["mcp", "--path", "/abs/path/to/project"]` (Cursor: `"${workspaceFolder}"`). Resolution order: `--path` → MCP `roots/list` → process `cwd` (last resort), walking up for `.crud/setup.toml` with a home-directory ceiling when the start path is under `$HOME`.

## Workflow

1. Call `crud_describe_templates` (and read `crud://` resources for docs)
2. Author `entity.json`
3. `crud_preview` (validates `entity.json` and returns its normalized field table for user confirmation; no code is rendered or written) → `crud_generate`

See [zh-CN/mcp-server.md](zh-CN/mcp-server.md) for the full tool/resource/prompt tables and DDL prefix notes.
