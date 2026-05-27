---
phase: 01-foundation-setup
plan: 02
subsystem: config
tags: [rust, clap, inquire, setup-config, toml, value-enum]

requires:
  - phase: 01-01
    provides: ErrorEnvelope, cli feature gate, agent mode output
provides:
  - setup 子命令 clap 闭集 flag 面（D-08）
  - SetupConfig 单一构建与 toml::to_string_pretty 序列化（D-10）
  - 交互 inquire 向导与 flag 路径字节一致 TOML
  - deny_unknown_fields + defaults←file←flags 合并（CONF-03/04）
affects:
  - 01-foundation-setup plans 03–04
  - Phase 2 gen command

tech-stack:
  added: []
  patterns:
    - "SetupConfig::from_selections 为交互/非交互唯一构建入口"
    - "clap ValueEnum + kebab-case 与 serde 枚举对齐"
    - "try_parse_cli_or_help 区分 DisplayHelp/Version 与 UserError"

key-files:
  created:
    - src/cli/args.rs
    - src/cli/setup_wizard.rs
    - src/core/config.rs
    - src/core/default_paths.rs
    - tests/setup_args_tests.rs
    - tests/setup_config_tests.rs
    - tests/setup_wizard_tests.rs
  modified:
    - src/cli/mod.rs
    - src/core/error.rs
    - src/core/mod.rs
    - src/main.rs
    - Cargo.toml

key-decisions:
  - "setup.toml 固定 [project]/[paths]/[overwrite] 三段；字段顺序为契约"
  - "仅序列化所选框架对应 path 键（D-11）"
  - "--help/--version 由 clap DisplayHelp/DisplayVersion 退出 0，不映射为 UserError"

patterns-established:
  - "Pattern: SetupSelections → SetupConfig::from_selections → to_toml_pretty"
  - "Pattern: UserError::user_error 携带 details.flag / details.value"

requirements-completed: [FOUND-04, CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-08]

duration: 13min
completed: 2026-05-27
---

# Phase 1 Plan 02: Setup 输入契约 Summary

**交互与非交互 setup 收敛到同一 SetupConfig，经 toml::to_string_pretty 产出确定性 TOML；clap 闭集枚举与 deny_unknown_fields 严格校验**

## Performance

- **Duration:** 13 min
- **Started:** 2026-05-27T10:32:45Z
- **Completed:** 2026-05-27T10:45:46Z
- **Tasks:** 3
- **Files modified:** 12

## Accomplishments

- `crud-cli setup` 子命令注册 `--backend/--frontend/--component-library/--overwrite-policy/--force`（D-08/CONF-08）
- 非法枚举与缺失必填 flag → `UserError(1)` + `details.flag/value`（D-09）
- `SetupConfig` 三段 schema、`deny_unknown_fields`、defaults←file←flags 合并（CONF-03/04）
- 框架路径默认映射 spring-boot/nest/vue/react（D-11/CONF-05）
- `inquire` 向导四维提示与 flag 路径 `setup_config_byte_identical` 测试通过（CONF-01/D-10）

## Task Commits

1. **Task 1+2: 参数面与 SetupConfig 管线** - `ae8e5ec` (feat)
2. **Task 3: 交互式 setup 向导** - `62cbd66` (feat)

_Note: Task 1 与 Task 2 同提交以保持 `args`↔`config` 编译依赖；逻辑上仍分两层交付。_

## Files Created/Modified

- `src/cli/args.rs` — 根 Parser、`setup` 子命令、ValueEnum、clap→UserError
- `src/core/config.rs` — `SetupConfig`、合并、`to_toml_pretty`
- `src/core/default_paths.rs` — D-11 路径默认
- `src/cli/setup_wizard.rs` — inquire Select 四维
- `tests/setup_*_tests.rs` — 11 项契约测试

## Decisions Made

- `--help`/`--version` 在 `try_parse_cli_or_help` 中单独处理，避免误判为 UserError
- 写盘与 `fs_writer` 留待后续 plan；本 plan 仅验证序列化路径
- Task 1/2 合并为单次 feat 提交（构建依赖），Task 3 独立提交

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] clap `--version` 被映射为 UserError**
- **Found during:** Task 1 验证
- **Issue:** `try_parse_cli` 将所有 clap 错误转为 UserError，导致 `--version` 退出码 1
- **Fix:** 新增 `try_parse_cli_or_help`，DisplayHelp/DisplayVersion 打印后正常退出
- **Files modified:** `src/cli/args.rs`, `src/main.rs`
- **Commit:** `ae8e5ec`

**2. [Rule 3 - Blocking] help 测试断言子命令级 flag**
- **Found during:** Task 1 验收
- **Issue:** `--backend` 在 `setup` 子命令 help 中，非根 help
- **Fix:** `cli_help_version_smoke` 改为检查 `setup` 子命令 render_help
- **Commit:** `ae8e5ec`

## Issues Encountered

- 计划要求每任务独立提交；Task 1 与 Task 2 因 `args::to_setup_config` 依赖 `core::config` 合并为 `ae8e5ec`。

## User Setup Required

None.

## Next Phase Readiness

- Plan 01-03+ 可接入 `fs_writer` 原子写 `.crud/setup.toml`
- `force-only` 策略与 `--force` 写盘语义在写路径 plan 中落地

## Self-Check: PASSED

- FOUND: src/cli/args.rs
- FOUND: src/core/config.rs
- FOUND: src/cli/setup_wizard.rs
- FOUND: tests/setup_args_tests.rs
- FOUND: commit ae8e5ec
- FOUND: commit 62cbd66

---
*Phase: 01-foundation-setup*
*Completed: 2026-05-27*
