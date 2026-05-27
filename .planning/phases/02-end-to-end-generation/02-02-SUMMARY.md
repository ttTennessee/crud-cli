---
phase: 02-end-to-end-generation
plan: 02
subsystem: codegen
tags: [rust, serde_path_to_error, gray_matter, globset, strsim, contract-tests]

requires:
  - phase: 02-end-to-end-generation
    plan: 01
    provides: gen_pipeline, GenRunParams, resolve_output_path layer-3, GenInput
provides:
  - JSON --file loader with closed-set UserError reasons and did-you-mean
  - setup.toml [variables] and [templates.outputs]
  - Front-matter + outputs-map + source-mirror resolve_output_path
  - --type filter, --dry-run listing, gen contract tests
affects: [02-03]

tech-stack:
  added: [serde_path_to_error 0.1, strsim 0.11, gray_matter 0.2, globset 0.4]
  patterns: [AsContextField trait, per-target overwrite preflight, DryRunLine report]

key-files:
  created:
    - src/core/template_meta.rs
    - tests/gen_input_tests.rs
    - tests/variables_blacklist_tests.rs
    - tests/template_meta_tests.rs
    - tests/gen_filter_tests.rs
    - tests/gen_outputs_map_tests.rs
    - tests/gen_frontmatter_tests.rs
    - tests/gen_dry_run_tests.rs
    - tests/contracts/gen_atomic_batch.rs
    - tests/contracts/gen_agent_stdout.rs
  modified:
    - src/core/config.rs
    - src/core/gen_input.rs
    - src/core/gen_context.rs
    - src/core/gen_pipeline.rs
    - src/core/template_loader.rs
    - src/cli/commands/gen.rs
    - src/cli/output.rs
    - src/core/gen_report.rs
    - Cargo.toml

key-decisions:
  - "Closed front-matter with invalid YAML returns TemplateError instead of silent layer-3 fallback"
  - "Front-matter and outputs-map path templates must quote Handlebars ({{) in YAML"
  - "load_gen_input_with_specs_from_json preserves FieldSpec extra for context"

patterns-established:
  - "Per-target overwrite preflight before batch commit with Always plan gate"
  - "emit_dry_run_listing: stdout in non-agent, tracing::info under agent"

requirements-completed: [GEN-02, GEN-06, GEN-08, GEN-09, GEN-10]

duration: 90min
completed: 2026-05-27
---

# Phase 2 Plan 02: Gen UX and Contracts Summary

**JSON `--file`, front-matter/output-map paths, `--type` filter, `--dry-run` listing, and atomic-batch plus agent stdout contracts for `gen`.**

## Performance

- **Duration:** ~90 min
- **Tasks:** 3/3
- **Files modified:** 25+

## Accomplishments

- `load_gen_input_from_json` with `serde_path_to_error`, six closed `UserError` reasons, Levenshtein did-you-mean on `unknown_field`
- `SetupConfig` extended with `[variables]` / `[templates.outputs]` (byte-identical when empty)
- `split_front_matter` + three-layer `resolve_output_path` (front-matter → outputs map → source mirror)
- `--type` globset filter with `template_type_not_found` hints
- `--dry-run` path listing with `[CONFLICT]` markers; zero writes
- Contract tests: `gen_atomic_batch` (SC#5), `gen_agent_stdout` (SC#4)

## Task Commits

1. **Task 1: setup.toml extensions + JSON loader** - `ff96980` (feat)
2. **Task 2: front-matter, type filter, outputs map** - `c396454` (feat)
3. **Task 3: dry-run, contract tests** - `c1864c4` (feat)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Critical] Invalid front-matter must not silently fall back to layer 3**
- **Found during:** Task 2
- **Issue:** Unquoted `{{` in YAML breaks yaml-rust; empty `Pod` previously fell through to source-mirror.
- **Fix:** `has_closed_front_matter` → `TemplateError`; document quoted Handlebars in templates.
- **Files:** `src/core/template_meta.rs`

**2. [Rule 2 - Critical] `load_gen_input_with_specs_from_json` for JSON `extra` fields**
- **Found during:** Task 2
- **Issue:** `GenInput.fields: Vec<Field>` cannot carry JSON `extra` map alone.
- **Fix:** Bundle loader used by pipeline; `AsContextField` on `FieldSpec`.
- **Files:** `src/core/gen_input.rs`, `src/core/gen_context.rs`, `src/core/gen_pipeline.rs`

**3. [Rule 1 - Bug] Test fixtures using `writeln!` stripped `{{` for TOML paths**
- **Found during:** Task 2 verification
- **Issue:** `writeln!` treats `{{` as format escape → single `{` on disk.
- **Fix:** `fs::write` with `const SETUP_TOML` in `gen_outputs_map_tests`.
- **Files:** `tests/gen_outputs_map_tests.rs`

### Deferred Issues

- `cargo clippy --features cli --tests -- -D warnings` reports 8 pre-existing errors (e.g. `setup_wizard.rs`, `template_engine` test module placement); unchanged from Plan 01 baseline.

## Self-Check

```
FOUND: src/core/gen_input.rs
FOUND: src/core/template_meta.rs
FOUND: tests/contracts/gen_atomic_batch.rs
FOUND: tests/contracts/gen_agent_stdout.rs
FOUND: ff96980
FOUND: c396454
FOUND: c1864c4
```

## Self-Check: PASSED
