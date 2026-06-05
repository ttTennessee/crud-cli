# MCP server

**Languages:** English · [简体中文](zh-CN/mcp.md)

`crud-cli` ships an embedded [Model Context Protocol](https://modelcontextprotocol.io/) server so AI agents can drive code generation through structured tool calls instead of shell invocations. The MCP server reuses the same `core` APIs as the CLI — same templates, same validation, same write semantics — exposed over stdio.

## Why use it

Compared to having the agent run `crud-cli gen ...` in a terminal:

- **Structured input / output.** Tools take JSON parameters and return JSON results; no need to parse human-readable CLI output.
- **Cheaper round-trips.** `crud_describe_templates` returns the variable / field-type schema of the active bundle in one call, so the agent doesn't have to discover it by reading files.
- **Preview before write.** `crud_preview` validates an `entity.json` and returns its normalized field table — the user can confirm field types and required flags before any file is written.
- **Same write guarantees.** `crud_generate` uses the same transactional two-phase write as the CLI; on conflict, nothing lands on disk.

## Build and launch

The MCP server is gated by the `mcp` Cargo feature. Use a prebuilt binary from [Releases](https://github.com/ttTennessee/crud-cli/releases) (built with `--features full`) or build locally:

```bash
cargo build --release --features full
./target/release/crud-cli mcp                # starts stdio server in current directory
./target/release/crud-cli mcp --path /abs/path/to/project
```

The server speaks MCP over **stdio** — it is not a long-running network daemon. The MCP client (Claude Desktop, Cursor, Cline, …) spawns it on demand.

### Project root resolution

Several tools need to know which project's `.crud/setup.toml` to use. Resolution order:

1. `--path <DIR>` argument (canonicalized; must exist and be a directory)
2. MCP `roots/list` from the client (if the client advertises roots)
3. Process `cwd`, walking upward for `.crud/setup.toml`, with `$HOME` as a ceiling when the start path is under `$HOME`

`--path` is the most explicit and is recommended for all client configs.

## Client configuration

### Claude Desktop / Claude Code (`claude_desktop_config.json`)

```json
{
  "mcpServers": {
    "crud-cli": {
      "command": "crud-cli",
      "args": ["mcp", "--path", "/abs/path/to/your/project"]
    }
  }
}
```

### Cursor (`.cursor/mcp.json` in the workspace)

```json
{
  "mcpServers": {
    "crud-cli": {
      "command": "crud-cli",
      "args": ["mcp", "--path", "${workspaceFolder}"]
    }
  }
}
```

### Cline / Continue / other generic MCP clients

Same shape — point `command` at the `crud-cli` binary on `PATH` and pass `["mcp", "--path", "<abs-project-dir>"]` as `args`. If the client supports MCP `roots/list` you may omit `--path`, but explicit is safer.

## Tools

| Name | Purpose |
|---|---|
| `crud_describe_templates` | Return the active bundle's `_variables.toml` schema, `_field_types.toml` aliases, project paths (`paths.lang` / `paths.aux`), and resolved project metadata. Call this **first** when authoring an `entity.json`. |
| `crud_preview` | Validate an `entity.json` and return its normalized field-by-field structure as a confirmation table. No files are rendered or written. |
| `crud_generate` | Validate and write generated files into the project tree. Supports a `type` filter (e.g. `ddl`) and `force` to bypass overwrite policy. Uses the same transactional write as `crud-cli gen`. |

Parameter and return-value schemas are surfaced via MCP `tools/list` — refer to your client's tool inspector for the live JSON schema.

## Resources

| URI | MIME | Content |
|---|---|---|
| `crud://schema/entity_guide` | `text/markdown` | The `entity.json` schema spec as a single markdown document. Same source as [`agent-resources/entity-json-guide.md`](../agent-resources/entity-json-guide.md). |
| `crud://schema/builtins` | `application/json` | Reserved variable / field identifier names that templates inject automatically. Useful for clients that want to validate `entity.json` locally before calling `crud_preview`. |

## Prompts

| Name | Purpose |
|---|---|
| `crud_template_authoring` | One-shot prompt returning the full template authoring guide as a user message. Same source as [`agent-resources/template-authoring.md`](../agent-resources/template-authoring.md). Useful when the agent is about to write or modify `.hbs` files. |

## Recommended agent workflow

For generating code into an existing project:

1. **`crud_describe_templates`** → learn the active bundle's variables, field types, and the project's path layout.
2. **(optional) Read `crud://schema/entity_guide`** if the agent isn't already familiar with the `entity.json` shape.
3. **Compose `entity.json`** based on the user's intent and the schema from step 1.
4. **`crud_preview`** → show the user the normalized field table for confirmation. Iterate if needed.
5. **`crud_generate`** → write files.

For authoring or adapting a template bundle:

1. **`crud_template_authoring` prompt** → loads the authoring guide into the conversation.
2. Read the target project's existing hand-written files (controllers, services, frontend pages) to capture house style.
3. Edit `.hbs` files in `.crud/templates/`.
4. Run `crud-cli validate` locally before generating.

## Error handling

Tool calls that fail validation return a structured JSON error in the tool's result body (not as an MCP protocol error). The shape matches the CLI's `--agent` error envelope:

```json
{
  "code": "...",
  "message": "...",
  "flag": "...",
  "value": "...",
  "remediation": "...",
  "details": { /* context-specific */ }
}
```

Agents can feed these back to the model as-is; the `remediation` field is written to be actionable.

Hard failures — server can't find the project root, can't read `setup.toml`, etc. — surface as MCP protocol errors with a human-readable message.

## See also

- [`agent-resources/template-authoring.md`](../agent-resources/template-authoring.md) — spec served by the `crud_template_authoring` prompt
- [`agent-resources/entity-json-guide.md`](../agent-resources/entity-json-guide.md) — spec served at `crud://schema/entity_guide`
- [Main README](../README.md) — project overview, install, basic CLI usage
