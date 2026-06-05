# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
While the project is on the 0.x line, breaking changes to agent-facing surfaces
(CLI flags, MCP tools/resources) may land in minor releases.

## [Unreleased]

## [0.1.2] - 2026-06-06

### Added
- **`_field_extra.toml` schema for `fields[].extra` validation.** Template
  bundles can now declare valid keys for `fields[].extra` in entity JSON,
  including value type, description, and which field types require them.
  Validation surfaces soft warnings for unknown or missing-required extra keys.
  New module `src/core/field_extra.rs` (`FieldExtraSchema`, `FieldExtraDef`,
  `ExtraValueType`, `load_schema`, `validate_extra_keys`); 11 tests in
  `tests/field_extra_schema_tests.rs`. Docs updated across `templates`,
  `entity`, `template-authoring`, and `entity-json-guide` (EN + zh-CN).
- **`crud://schema/entity_example` MCP resource.** `list_resources` now scans
  `templates_root` for `_example*.json` files and exposes them as a single
  merged JSON array (sorted by filename). Omitted when no example files exist,
  so existing bundles need no changes.
- **`crud_entity_schema` MCP tool.** Tool-only MCP clients (opencode,
  cursor-cli, …) cannot read MCP resources. The new tool takes a `name`
  parameter and exposes the same content as the `crud://schema/*` resources,
  making it reachable from every client.

### Changed
- **MCP `crud_preview` → `crud_validate`.** Agents were misusing `crud_preview`
  as a display tool, causing repeated calls and unnecessary token waste.
  `crud_validate` has a single clear contract: validate `entity.json` and
  return `ok=true` (with optional `warnings[]`) or an error — nothing more.
  All markdown-rendering helpers (`build_field_section`, `table_header`, …)
  and `tool_preview_result` were removed. Docs updated across the project.
- **MCP resource `crud://schema/entity` → `crud://schema/entity_guide`.**
  Renamed for clarity ahead of folding into `crud_entity_schema`.

### Removed
- **MCP resources `crud://schema/entity_guide`, `crud://schema/entity_example`,
  `crud://schema/builtins`.** Folded into the new `crud_entity_schema` tool so
  tool-only clients can reach the same content. The `crud_template_authoring`
  prompt is left untouched since it targets template authors and is typically
  user-initiated.

### Migration notes (MCP clients)
- Replace any call to `crud_preview` with `crud_validate`. Output shape is now
  `{ ok: true, warnings?: [...] }` on success instead of a rendered preview.
- Replace reads of `crud://schema/entity`, `crud://schema/entity_example`, and
  `crud://schema/builtins` with calls to the `crud_entity_schema` tool, passing
  `name` ∈ {`entity_guide`, `entity_example`, `builtins`}.

## [0.1.1] - prior release

See git history.

## [0.1.0] - initial release

See git history.

[Unreleased]: https://github.com/ttTennessee/crud-cli/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/ttTennessee/crud-cli/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/ttTennessee/crud-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/ttTennessee/crud-cli/releases/tag/v0.1.0
