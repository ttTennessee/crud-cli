# Roadmap: crud-cli

**Created:** 2026-05-27
**Mode:** mvp
**Granularity:** coarse
**Core Value:** 让 AI Agent 用一条几十字符的命令在本地瞬间渲染出与项目代码风格一致的全套 CRUD 文件，彻底消除 Agent 输出模板代码的 Token 浪费和风格漂移

## Phases

- [x] **Phase 1: Foundation + Setup** — Agent-friendly CLI discipline 落地，`crud-cli setup` 能交互/非交互生成 `.crud/setup.toml`（原子写） (completed 2026-05-27)
- [ ] **Phase 2: End-to-End Generation** — `crud-cli gen` 从 `.crud/templates/` 渲染 CRUD 文件，`validate` 校验模板，跑通第一次真正用户价值
- [ ] **Phase 3: Community Templates** — 双层模板加载 + `template install/list/remove`，安全从 GitHub 拉社区模板
- [ ] **Phase 4: Release** — 三平台 single-binary artifact + README 完整示例

## Phase Details

### Phase 1: Foundation + Setup

**Goal:** 用户可运行 `crud-cli setup` 在项目里生成 `.crud/setup.toml`，CLI 的 Agent-friendly 契约（退出码、stderr 分流、no-escape、panic hook、原子写）从第一行代码就成立
**Mode:** mvp
**Depends on:** Nothing (first phase)
**Requirements:** FOUND-01, FOUND-02, FOUND-03, FOUND-04, FOUND-05, FOUND-06, FOUND-07, FOUND-08, FOUND-09, FOUND-10, CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-06, CONF-07, CONF-08, CONF-09, CONF-10
**Success Criteria** (what must be TRUE):

  1. `cargo build --release` 产出 `crud-cli` 二进制；`crud-cli --help` 与 `crud-cli --version` 在 Linux / macOS / Windows CI 全绿
  2. `crud-cli setup` 交互式向导（`inquire`）可问完后端/前端/组件库/覆盖策略并把结果写到 `<cwd>/.crud/setup.toml`；同等 flags 的非交互模式产出 byte-identical 文件
  3. `cargo check --no-default-features --lib` 通过且 `examples/library_usage.rs` 在 CI 里编译运行，证明 `core::*` 不依赖 `cli::*`
  4. 写 `.crud/setup.toml` 时若目标存在且 `allow_overwrite=false`，整批不写并以退出码 3 + 结构化 stderr 报告；`--agent` 模式下成功 stdout 为空
  5. 强制注入故障：触发 `panic!()` 的测试得到退出码 99 + 结构化 stderr（非默认 101）；Handlebars 单测验证 `<>&` 不被 HTML-escape

**Plans:** 4/4 plans complete
**Wave 1**

- [x] 01-01-PLAN.md — Lock process-level CLI contract (error envelope, panic→99, agent mode transport) and bootstrap core entry modules

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — Implement setup input surface (interactive + flags) and canonical SetupConfig serialization/merge path

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 01-03-PLAN.md — Deliver transactional `.crud/setup.toml` writer with conflict/overwrite policy enforcement

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 01-04-PLAN.md — Add no-escape/core-boundary contract tests and tri-OS CI + clippy discipline gates

### Phase 2: End-to-End Generation

**Goal:** 用户可用 `crud-cli gen User --fields "id:Long,name:String" --package com.x --table user` 从 `.crud/templates/` 渲染出落盘的 CRUD 文件，并用 `crud-cli validate` 提前发现模板错误
**Mode:** mvp
**Depends on:** Phase 1
**Requirements:** GEN-01, GEN-02, GEN-03, GEN-04, GEN-05, GEN-06, GEN-07, GEN-08, GEN-09, GEN-10, VAL-01, VAL-02, VAL-03, VAL-04
**Success Criteria** (what must be TRUE):

  1. 给定 `.crud/templates/Entity.java.hbs`（含 YAML front-matter `basePath: src/main/java/{{package_path}}`），运行 `crud-cli gen User --fields "id:Long,name:String" --package com.acme.demo --table sys_user` 产出可编译的 `src/main/java/com/acme/demo/User.java`，内含未转义的 `List<…>` 与正确 case 转换
  2. `crud-cli gen --file user.json` 接收等价 JSON 输入产出 byte-identical 结果；`--dry-run` 仅打印待生成清单不落盘
  3. `crud-cli validate` 在含语法错误（未闭合 `{{#if}}`）或未声明变量（`{{nonexistent}}`）的模板上以退出码 2 失败，stderr 含 `template_path`/`line`/`column`/`variable`/`suggestion` 字段
  4. `--agent` 模式下 `gen` 成功时 stdout 仅打印一行规范文案（含生成文件数），失败按退出码 + 结构化 stderr；`GenReport` 数据结构含写入路径/跳过项/冲突项，便于未来 `--json` 复用
  5. 模板中任一文件冲突（已存在且未 `--force`）触发整批不写（两阶段 plan/commit），退出码 3

**Plans:** 3 plans
Plans:
**Wave 1**

- [ ] 02-01-PLAN.md — End-to-end `gen` happy-path vertical slice (--fields DSL → render → fs_writer atomic write)

**Wave 2** *(blocked on Wave 1 completion)*

- [ ] 02-02-PLAN.md — JSON --file input, front-matter, [templates.outputs] map, --type filter, --dry-run UX, atomic-batch contract test

**Wave 3** *(blocked on Wave 2 completion)*

- [ ] 02-03-PLAN.md — `validate` command: syntax + variable first-segment + synthetic-fixture render + aggregated TemplateError(2) envelope

### Phase 3: Community Templates

**Goal:** 用户可用 `crud-cli template install user/ruoyi-template` 从 GitHub 拉社区模板到 `~/.crud/templates/`，并在 `gen` 时让项目模板优先于同名全局模板
**Mode:** mvp
**Depends on:** Phase 2
**Requirements:** TPL-01, TPL-02, TPL-03, TPL-04, TPL-05, TPL-06, TPL-07, TPL-08, TPL-09
**Success Criteria** (what must be TRUE):

  1. `crud-cli template install <user>/<repo>` 通过 `codeload.github.com` 下载 tarball，抽取到 `~/.crud/templates/<name>/`，并把 commit SHA 与来源 repo 记到 `.crud/state.json`；`crud-cli template list` 显示同样信息
  2. 含 `../../../etc/passwd`、绝对路径、symlink、设备文件的恶意 tar 在抽取阶段被全部拒绝，退出码 1，且 `~/.crud/templates/` 之外无任何写入
  3. `gen` 命令在项目模板 + 全局模板共存时按 per-file 路径合并（项目优先），并在 stderr 打印 per-file 解析图（来自项目 vs 全局）
  4. `GITHUB_TOKEN` 存在时被读入提升速率限制；任何日志中 token 被 redact（CI grep 确认）
  5. `crud-cli template remove <name>` 干净删除该模板目录与 `.crud/state.json` 中对应记录

**Plans:** TBD

### Phase 4: Release

**Goal:** 用户可从 GitHub Release 拿到三平台 single-binary 并按 README 一步步跑通 `setup → gen → validate → template install` 全流程
**Mode:** mvp
**Depends on:** Phase 3
**Requirements:** REL-01, REL-02
**Success Criteria** (what must be TRUE):

  1. CI 矩阵在 Linux / macOS / Windows 上 `cargo build --release` 均产出单二进制 artifact 并附加到 GitHub Release
  2. README.md 含可复制粘贴的安装步骤 + `setup`、`gen`、`validate`、`template install` 端到端示例，按示例顺序执行无错误

**Plans:** TBD

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Foundation + Setup | 4/4 | Complete   | 2026-05-27 |
| 2. End-to-End Generation | 0/3 | Planned | - |
| 3. Community Templates | 0/0 | Not started | - |
| 4. Release | 0/0 | Not started | - |

## Coverage Summary

- v1 requirements: 45 total
- Mapped to phases: 45
- Unmapped: 0 ✓

---
*Roadmap created: 2026-05-27*
