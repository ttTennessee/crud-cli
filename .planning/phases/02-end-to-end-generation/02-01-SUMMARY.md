---
phase: 02-end-to-end-generation
plan: 01
subsystem: codegen
tags: [rust, clap, handlebars, ignore, convert_case, mvp-slice]

requires:
  - phase: 01-foundation-setup
    provides: ErrorEnvelope, fs_writer plan/commit, template_engine no_escape, setup.toml loader
provides:
  - crud-cli gen command with --fields DSL happy path
  - field_dsl, template_loader, gen_context, gen_pipeline, GenReport surfaces
  - End-to-end test seeding .crud/templates/Entity.java.hbs
affects: [02-02, 02-03]

tech-stack:
  added: [convert_case 0.6, ignore 0.4]
  patterns: [GenRunParams core boundary, ignore WalkBuilder + .crudignore, closed-set DSL reasons]

key-files:
  created:
    - src/cli/commands/gen.rs
    - src/core/field_dsl.rs
    - src/core/git_info.rs
    - src/core/gen_input.rs
    - src/core/gen_context.rs
    - src/core/gen_run.rs
    - src/core/gen_pipeline.rs
    - src/core/gen_report.rs
    - src/core/template_loader.rs
    - tests/gen_e2e_happy_path.rs
    - tests/gen_args_tests.rs
    - tests/field_dsl_tests.rs
    - tests/template_loader_tests.rs
    - tests/gen_context_tests.rs
    - tests/gen_pipeline_tests.rs
  modified:
    - src/cli/args.rs
    - src/core/error.rs
    - src/core/template_engine.rs
    - src/main.rs
    - Cargo.toml

key-decisions:
  - "GenRunParams in core keeps gen_pipeline free of clap (FOUND-02)"
  - "Parallel gen tests serialize cwd changes with a static Mutex"

patterns-established:
  - "user_error_with_reason for closed-set gen/DSL failure codes"
  - "D-G28 layer-3 output path via resolve_output_path before fs_writer"

requirements-completed: [GEN-01, GEN-03, GEN-04, GEN-05, GEN-07, GEN-09, GEN-10]

duration: 45min
completed: 2026-05-27
---

# Phase 2 Plan 01: End-to-End Gen Happy Path Summary

**`crud-cli gen` renders a single `.crud/templates/*.hbs` to disk with DSL fields, case helpers, no HTML escape, and Chinese success line.**

## Performance

- **Duration:** ~45 min
- **Tasks:** 5/5
- **Files modified:** 20+

## Accomplishments

- `GenArgs` + `Commands::Gen` wired through `run_gen` → `gen_pipeline::run`
- `--fields` micro-DSL with 7 closed `reason` codes and reserved-name blacklist
- `discover_templates` via `ignore::WalkBuilder` + `.crudignore`
- `build_context` with model/field case suffixes and `git_user_*`
- Full pipeline: render → `resolve_output_path` → `fs_writer::plan/commit`
- `gen_e2e_happy_path` GREEN

## Task Commits

1. **Task 1: Failing e2e + GenArgs wiring** - `51542cb` (test)
2. **Task 2: DSL + case helpers + git_info** - `fbb8c2c` (feat)
3. **Task 3: template_loader + gen_context** - `c398364` (feat)
4. **Task 4: resolve_output_path** - `792c2e1` (feat) — path resolver + unit tests
5. **Task 5: gen_pipeline orchestrator** - `792c2e1` (feat) — same commit as Task 4 (orchestrator + e2e GREEN)

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Critical] Introduced `GenRunParams` in `core/gen_run.rs`**
- **Found during:** Task 5
- **Issue:** Plan placed `gen_pipeline::run(GenArgs)` in `core`, but `GenArgs` lives under `cli` (clap); breaks `cargo check --no-default-features --lib` (FOUND-02).
- **Fix:** Core `gen_pipeline::run(GenRunParams)`; `run_gen` maps `GenArgs` → `GenRunParams` in `cli/commands/gen.rs`.
- **Files:** `src/core/gen_run.rs`, `src/cli/commands/gen.rs`, `src/core/gen_pipeline.rs`

**2. [Rule 1 - Bug] Cwd lock for parallel gen integration tests**
- **Found during:** Task 5 verification (`cargo test --features cli`)
- **Issue:** `gen_e2e_happy_path` and `gen_pipeline_tests` raced on `set_current_dir`.
- **Fix:** Static `CWD_LOCK` mutex around chdir blocks (same pattern as `setup_write_tests`).
- **Files:** `tests/gen_e2e_happy_path.rs`, `tests/gen_pipeline_tests.rs`

## Self-Check

```
FOUND: src/cli/commands/gen.rs
FOUND: src/core/field_dsl.rs
FOUND: src/core/gen_pipeline.rs
FOUND: tests/gen_e2e_happy_path.rs
FOUND: 51542cb
FOUND: fbb8c2c
FOUND: c398364
FOUND: 792c2e1
```

## Self-Check: PASSED
