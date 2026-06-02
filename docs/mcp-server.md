# MCP Server (`crud-cli mcp`)

> **Other languages:** [简体中文](zh-CN/mcp-server.md)

Exposes `crud-cli` code generation to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io/). Single entry point: **`crud-cli mcp`**.

## Build

```bash
cargo build --release --features full
crud-cli mcp
```

Configure your MCP client with `command`: `crud-cli`, `args`: `["mcp"]`, and `cwd` set to your project root (must contain `.crud/setup.toml` and templates).

## Workflow

1. Read `crud://` resources or call `describe_templates`
2. Author `entity.json`
3. `validate_entity` → `preview` (optional, `type=ddl` for DDL only) → `generate`

See [zh-CN/mcp-server.md](zh-CN/mcp-server.md) for the full tool/resource/prompt tables and DDL prefix notes.
