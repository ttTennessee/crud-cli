# Documentation

**Languages:** English · [简体中文](zh-CN/README.md)

Human-facing documentation. Machine-readable specs served to LLM agents live in [`../agent-resources/`](../agent-resources/).

| Path | Audience | Status |
|---|---|---|
| [quickstart.md](./quickstart.md) | New users — install, basic usage, CLI subcommand reference | English |
| [templates.md](./templates.md) | Template authors | Placeholder — full guide pending |
| [entity.md](./entity.md) | `entity.json` schema reference | English |
| [mcp.md](./mcp.md) | MCP integrators — server config, tools, resources, prompts | English |
| [dev/](./dev/) | Contributors (`crud-cli` development) | — |

Chinese mirrors of all four user-facing docs above live under [`zh-CN/`](./zh-CN/).

## Machine-readable specs

These live outside `docs/` because they are embedded into the binary via `include_str!` and served as MCP resources/prompts to LLM agents. They follow different writing rules (terse spec, no human prose).

| Path | Served as |
|---|---|
| [../agent-resources/template-authoring.md](../agent-resources/template-authoring.md) | MCP prompt `crud_template_authoring` |
| [../agent-resources/entity.md](../agent-resources/entity.md) | MCP resource `crud://schema/entity` |
