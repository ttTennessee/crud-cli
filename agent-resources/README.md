# agent-resources/

Markdown in this directory is **embedded into the `crud-cli` binary via
`include_str!`** and served to LLM agents through the MCP server
(`src/mcp/server.rs`, `src/mcp/resources.rs`).

## Writing rules

- **Audience is an LLM, not a human.** No prose warm-ups, no tradeoff
  discussions, no "see also" tangents.
- **Every token costs money.** Cut anything a model can infer from the spec.
- **Be exhaustive on the machine-readable parts**: field names, enum values,
  precedence rules, error conditions. Models hallucinate when these are vague.
- **English only.** No `zh-CN/` mirror — LLMs handle English specs more
  reliably and one canonical version avoids drift.
- **Examples must be copy-pasteable.** Prefer fenced code blocks over prose
  description of syntax.

## Files

| File | MCP surface |
|---|---|
| `template-authoring.md` | Prompt `crud_template_authoring` |
| `json-entity-input.md`  | Resource `URI_ENTITY_SCHEMA` |

Human-facing documentation lives in `docs/`.
