---
phase: 02-end-to-end-generation
plan: 03
subsystem: codegen
tags: [rust, handlebars, validator, strsim, validate, contract-tests]

requires:
  - phase: 02-end-to-end-generation
    plan: 01
    provides: template_loader, template_engine, gen_context, ErrorEnvelope
  - phase: 02-end-to-end-generation
    plan: 02
    provides: setup.toml [variables] in synthetic fixture context
provides:
  - crud-cli validate with VAL-01..04 checks and aggregated TemplateError(2) envelope
  - Agent-mode validate contract (silent success, JSON stderr on failure)
affects: []

tech-stack:
  added: []
  patterns: [per-template fail-fast, cross-template issue aggregation, handlebars AST first-segment walk]

key-files:
  created:
    - src/core/validator.rs
    - src/cli/commands/validate.rs
    - tests/validate_e2e_tests.rs
    - tests/validate_syntax_tests.rs
    - tests/validate_unknown_var_tests.rs
    - tests/validate_render_tests.rs
    - tests/contracts/validate_agent_stdout.rs
  modified:
    - src/cli/args.rs
    - src/cli/mod.rs
    - src/cli/commands/mod.rs
    - src/main.rs
    - src/core/error.rs
    - src/core/mod.rs
    - Cargo.toml

key-decisions:
  - "Core uses ValidateParams instead of cli::ValidateArgs to preserve FOUND-02 (no-default-features lib without cli)"
  - "Variable allow-set uses handlebars 6.4 TemplateElement AST walk (not regex fallback); each-fields scope pushes field-object keys on #each fields blocks"
  - "Local path segments normalize index/key/first/last to @-prefixed allow-set entries"

patterns-established:
  - "validate::run mirrors gen template discovery and build_context synthetic fixture (3 hardcoded fields + setup [variables])"
  - "template_error_with_issues carries details.summary and details.issues for D-G23"

requirements-completed: [VAL-01, VAL-02, VAL-03, VAL-04]

duration: 45min
completed: 2026-05-27
---

# Phase 2 Plan 03: Validate Command Summary

**`crud-cli validate` aggregates syntax, unknown-variable, and synthetic-fixture render issues across all templates into a single TemplateError(2) envelope with did-you-mean hints.**

## Performance

- **Duration:** 45 min
- **Tasks:** 3
- **Files modified:** 13

## Accomplishments

- `validate` subcommand with `--type` filter parity to `gen`
- Per-template fail-fast: syntax → variables → render; cross-template aggregation with `details.summary` counts
- VAL-02 did-you-mean via `strsim::levenshtein` over built-ins + `[variables]` keys
- Agent contract: success silent; failure single-line JSON on stderr

## Task Commits

1. **Task 1: ValidateArgs + wiring + validator + happy-path test** - `191c021` (feat)
2. **Task 2: Syntax + unknown-variable tests** - `0dd6c30` (test)
3. **Task 3: Render + aggregate + agent contract tests** - `8d23c61` (test)

## Files Created/Modified

- `src/core/validator.rs` - Orchestrates discover → compile → variable walk → fixture render
- `src/cli/commands/validate.rs` - `run_validate` + `emit_success` line
- `src/core/error.rs` - `template_error_with_issues` factory
- `tests/validate_*` + `tests/contracts/validate_agent_stdout.rs` - VAL-01..04 coverage

## Deviations from Plan

### Auto-fixed / minor adjustments

**1. [Rule 2 - FOUND-02] `ValidateParams` in core instead of `ValidateArgs`**
- **Reason:** `validator::run` cannot take `cli::ValidateArgs` without coupling core to the `cli` feature
- **Fix:** `ValidateParams { type_filter }` in core; `run_validate` maps from CLI args

**2. [Rule 1 - Test fidelity] missing_helper fixture uses `{{date_helper model}}`**
- **Reason:** `{{date_helper x}}` fails VAL-02 on `x` before render reaches missing-helper detection
- **Fix:** Use allowed `model` param so render phase reports `missing_helper`

**3. AST walk vs regex**
- Used handlebars `TemplateElement` / `Parameter::Path` walk per plan; regex fallback not required (all variable tests green)

## Self-Check: PASSED

- FOUND: src/core/validator.rs
- FOUND: src/cli/commands/validate.rs
- FOUND: tests/validate_e2e_tests.rs
- FOUND: tests/contracts/validate_agent_stdout.rs
- FOUND: commit 191c021
- FOUND: commit 0dd6c30
- FOUND: commit 8d23c61
