<!-- GSD:project-start source:PROJECT.md -->
## Project

**crud-cli**

`crud-cli` 是一个用 Rust 编写的命令行工具，用于配合 AI Agent 高效生成后台管理系统的 CRUD 代码。通过把重复的模板本地化，让 Agent 只下发"命令 + 结构化数据"，由本地 CLI 渲染并落盘，从而把 Agent 的 Token 消耗降低约 40 倍（从 2000+ 降到 ~50）。面向使用 AI Agent（Cline、Copilot、Claude Code 等）进行后台管理系统开发的工程师。

**Core Value:** 让 AI Agent 用一条几十字符的命令，瞬间在本地渲染出与项目代码风格一致的全套 CRUD 文件（前后端），彻底消除 Agent 输出模板代码带来的 Token 浪费和风格漂移。

### Constraints

- **技术栈**：Rust（`cargo` 构建），单二进制分发 — 来自 PRD 设计
- **架构**：`core` 与 `cli` 严格分离，为后续 MCP Server 预留接口 — 来自 PRD 设计决策
- **兼容性**：项目模板路径固定为 `.crud/templates/`，全局模板路径固定为 `~/.crud/templates/<模板名>/`
- **配置**：项目配置文件位于 `<项目根>/.crud/setup.toml`（TOML 格式）
- **安全**：默认不覆盖已有文件 — 防止 Agent 误操作覆盖用户代码
- **模板引擎**：PRD 示例使用 `.hbs` 后缀，倾向 Handlebars 风格语法
<!-- GSD:project-end -->

<!-- GSD:stack-start source:research/STACK.md -->
## Technology Stack

## TL;DR
- **Workspace layout:** Cargo workspace with TWO library crates + one binary — `crates/crud-core` (pure logic), `crates/crud-cli` (thin clap layer + binary). This keeps the future MCP server a 4th crate (`crates/crud-mcp`) that depends on `crud-core` only.
- **CLI:** `clap` 4.6 (derive) — no real alternative.
- **Templates:** `handlebars` 6.4 — chosen because the PRD explicitly specifies `.hbs` files and Handlebars syntax (`{{model}}`). `minijinja` would be faster but breaks PRD compatibility.
- **Config:** plain `serde` + `toml` + `serde_json`. Skip `figment`/`config-rs` — they over-engineer a single-file `.crud/setup.toml`.
- **Filesystem walk:** `ignore` (the ripgrep crate) — gives `.gitignore` semantics for free, which template scanning needs.
- **GitHub fetching:** `ureq` + `flate2` + `tar` against `https://codeload.github.com/{user}/{repo}/tar.gz/HEAD`. Reject `git2` (libgit2 = C deps, breaks single-binary story) and `gix` (large compile time, overkill for "download a snapshot").
- **Errors:** `anyhow` in `crud-cli`, `thiserror` in `crud-core`. Canonical split.
- **Prompts:** `inquire` 0.9 — richer prompts (Select/MultiSelect/Confirm) and better validators than `dialoguer`.
- **Logging:** `tracing` + `tracing-subscriber` — future MCP server will want structured logs.
- **Testing:** `insta` for snapshotting rendered template output, `assert_cmd` + `predicates` + `tempfile` for end-to-end CLI tests.
## Recommended Stack
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `clap` (derive) | 4.6.1 | CLI parsing, subcommands, help, completion | De-facto standard. Powers `cargo`, `ripgrep`, `bat`, `fd`. Derive API is the recommended path in 2025/2026 per official tutorial. No serious competitor for a multi-subcommand CLI of this shape. |
| `handlebars` | 6.4.1 | Template rendering of `.hbs` files | **PRD explicitly specifies Handlebars syntax** (`{{model}}Controller.java.hbs`). Mature, used by rust-lang.org itself, supports custom helpers (needed for `snake_case`/`PascalCase`/`camelCase` filters). |
| `serde` + `serde_derive` | 1.0.228 | Universal (de)serialization | Required by every config/data crate below. |
| `toml` | 1.1.2 | Read/write `.crud/setup.toml` | Official `toml-rs` v1.x — TOML 1.1 spec, round-trips comments via `toml_edit` (re-exported). The 1.x line is stable. |
| `serde_json` | 1.0.150 | Read `gen --file <json>` input | Standard. |
| `anyhow` | 1.0.102 | Error type in `crud-cli` (binary boundary) | Standard pairing: `anyhow` for "I just want a `?` with context at the top of the binary", `thiserror` for typed library errors. |
| `thiserror` | 2.0.18 | Typed errors in `crud-core` | The 2.x line landed in late 2024; stable. Lets `crud-core` expose enum-shaped errors that MCP server can pattern-match. |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `inquire` | 0.9.4 | Interactive `crud-cli setup` prompts (Select / MultiSelect / Confirm / Text) | Use for all interactive prompts. Richer than `dialoguer`, better validators, derive macros for enums. |
| `ignore` | 0.4.25 | Walk `.crud/templates/` and `~/.crud/templates/<name>/` | Use instead of `walkdir`. Free `.gitignore`/`.crudignore` support; parallel walker; battle-tested by ripgrep. |
| `ureq` | 3.3.0 | Blocking HTTP GET of GitHub tarballs | Use for `template install`. Blocking-only, ~6 deps. Avoids tokio/async runtime entirely — keeps binary small and CLI startup fast. |
| `flate2` | 1.1.9 | Gunzip tarball streams | Pair with `tar`. Pure-Rust `miniz_oxide` backend (no zlib C dep). |
| `tar` | 0.4.46 | Untar tarball into `~/.crud/templates/<name>/` | Standard. Pair with `flate2::read::GzDecoder`. |
| `tempfile` | 3.27.0 | Atomic writes during template install & tests | Write tarball to temp dir, untar, then `rename` into final location. Avoids partial-install corruption. |
| `dirs` | 6.0.0 | Resolve `~/.crud/` cross-platform | Use `dirs::home_dir()`. Simpler than `directories` (which encodes XDG vs macOS vs Windows conventions — we deliberately want `~/.crud/` on all platforms per PRD). |
| `tracing` | 0.1.44 | Structured logging in `crud-core` | Use `tracing::{info, debug, warn, error}` in `crud-core`. MCP server will set up its own subscriber. |
| `tracing-subscriber` | 0.3.23 | Logging output in `crud-cli` | Wire to stderr with `EnvFilter::from_env("CRUD_LOG")`. Respects `--verbose` flag. |
| `serde_path_to_error` | 0.1.x | Better JSON/TOML deserialization errors | Optional but cheap — turns `expected string, found null` into `at .fields[0].type: expected string`. Huge UX win for `--file <json>` and `validate`. |
| `globset` | 0.4.x | Match template file globs (re-exported from `ignore`) | Used by template discovery & overwrite-policy filtering. |
| `unicode-segmentation` | 1.x | Safe case conversion in template helpers | For `snake_case`/`PascalCase` helpers — handles non-ASCII identifiers correctly. |
| `convert_case` | 0.6.x | Identifier case conversion | Drop-in helpers (`to_case(Case::Snake)`, etc.) for Handlebars `{{snake model}}` etc. |
### Development Tools
| Tool | Purpose | Notes |
|------|---------|-------|
| `insta` 1.47.2 | Snapshot tests for rendered templates | `insta::assert_snapshot!(render(template, ctx))`. Critical for catching template regressions. Use `cargo insta review` workflow. |
| `assert_cmd` 2.2.2 | End-to-end CLI tests (`crud-cli gen user ...`) | Spawn the actual binary, assert stdout/stderr/exit code. |
| `predicates` 3.1.4 | Assertions for `assert_cmd` | `predicate::str::contains("生成成功")`. |
| `tempfile` 3.27.0 | Per-test sandbox dirs | Each integration test gets a fresh `tempdir()` that becomes a fake project root. |
| `cargo-dist` (optional) | Cross-platform release binaries | Once 1.0 is shipped, use for `cargo dist init` to get GitHub Actions release pipelines (mac/linux/windows static binaries). |
| `cargo-nextest` | Faster test runner | 3-5× faster than `cargo test`, better output for CLI test suites. |
## Installation
# Cargo.toml (workspace root)
# crates/crud-core/Cargo.toml
# crates/crud-cli/Cargo.toml
## Project Skeleton (prescriptive)
### Why this layout (not single crate, not 4 crates today)
| Option | Verdict | Why |
|--------|---------|-----|
| Single crate with `cli/` + `core/` modules | **Reject** | Nothing enforces the boundary. `cli` will quietly seep into `core`. The PRD makes the MCP-reuse split a hard requirement. |
| 2-crate workspace (`crud-core` + `crud-cli`) | **Adopt** | Compiler enforces the boundary: `crud-core` cannot depend on clap/inquire. MCP server crate can be added later in 1 line. |
| 3-crate today (add empty `crud-mcp` stub) | **Reject for v1** | PRD explicitly defers MCP to post-v1. Empty crate adds maintenance burden with zero current value. |
## Alternatives Considered
| Recommended | Alternative | When to Use Alternative |
|-------------|-------------|-------------------------|
| `handlebars` | `minijinja` | If PRD's `.hbs` constraint is dropped — minijinja parses ~11× faster, renders ~1.3× faster, smaller dep tree. But: changes user-facing syntax, breaks every example in the PRD. |
| `handlebars` | `tera` | Tera 1.x is still maintained but unmistakably mature (last 2.x is alpha). Jinja-like syntax. Same incompatibility issue as minijinja. |
| `handlebars` | `askama`/`rinja` | These are **compile-time** template engines — templates are compiled into the binary. Impossible for this project: templates must be loaded at runtime from `.crud/templates/`. |
| `ureq` | `reqwest` | Use reqwest if the project later adds async network ops (e.g., MCP server over HTTP). For one-shot tarball download, ureq is dramatically lighter (no tokio). |
| `ureq` + `flate2` + `tar` | `git2` | Use git2 if you need true `git clone` semantics (branches, tags, history, auth via SSH). For "download HEAD snapshot of `user/repo`", git2 brings libgit2 C linkage that wrecks single-binary distribution. |
| `ureq` + `flate2` + `tar` | `gix` | Pure-Rust alternative to git2 but heavy: dozens of crates, slow compile, and per gix's own discussion thread "many advanced git workflow parts aren't implemented yet". Reach for it only if v2 needs in-tree git operations. |
| `inquire` | `dialoguer` | Use dialoguer if you already depend on the `console`/`indicatif` ecosystem and want a smaller dep tree. Inquire wins on prompt variety and ergonomics. |
| `inquire` | `cliclack` | cliclack is newer, prettier output. Worth re-evaluating in 6 months. Less mature today (smaller user base). |
| plain `toml`+`serde` | `figment` / `config-rs` | Use these when you need to **merge** config from multiple sources (env vars + CLI flags + file + defaults). `.crud/setup.toml` is a single file; the merge engines are pure overhead here. |
| `ignore` | `walkdir` | Use walkdir for simple recursive walks where you don't care about `.gitignore`. We do care (template authors will want a `.crudignore`), so `ignore` wins. |
| `tracing` | `log` + `env_logger` | log + env_logger is simpler but lacks structured fields and spans. MCP server will want spans for request tracing. Use tracing now to avoid migration. |
## What NOT to Use
| Avoid | Why | Use Instead |
|-------|-----|-------------|
| `structopt` | Merged into clap 3+ years ago. Deprecated. | `clap` 4.x with `derive` feature. |
| `clap` 3.x | EOL. Missing derive ergonomics. | `clap` 4.6.x. |
| `git2` for "install template from GitHub" | C-linked libgit2 breaks single-binary distribution; massively over-spec for snapshot download. | `ureq` + `flate2` + `tar` against `codeload.github.com`. |
| `gix` for v1 | Compile time + immature surface area; not yet a clone-and-go solution. | Same as above (ureq+flate2+tar). Revisit for v2 if real git ops needed. |
| `figment` / `config-rs` for single-file config | Layered config engines are over-engineered for one TOML file. | `serde` + `toml`. |
| `chrono` (default features) | Pulls timezone DB; large. | `time` 0.3 if dates needed; or `chrono` with `default-features = false`. |
| `failure` | Long-deprecated. | `anyhow` + `thiserror`. |
| `error-chain` | Long-deprecated. | `anyhow` + `thiserror`. |
| `lazy_static` | Use `std::sync::OnceLock` (stable since 1.70) or `once_cell`. | `OnceLock` / `LazyLock` (stable 1.80). |
| Async runtime (tokio/async-std) in v1 | Adds ~50 deps, slows CLI startup, complicates error handling. CLI is sync; one HTTP call doesn't need async. | Blocking `ureq`. Revisit only if MCP server uses async transport. |
| `serde_yaml` | Unmaintained as of 2024. | If YAML ever needed: `serde_yml` (community fork) or `serde_yaml_ng`. Not needed for v1. |
| `prettytable-rs` for `template list` | Unmaintained. | `comfy-table` or just `println!` formatted output. |
## Stack Patterns by Variant
- Keep ureq+flate2+tar, but accept `user/repo[@ref]` syntax and an `Authorization: Bearer $GITHUB_TOKEN` env var (read from `GITHUB_TOKEN` / `GH_TOKEN`).
- Codeload URL becomes `https://codeload.github.com/{user}/{repo}/tar.gz/{ref}`.
- Do NOT switch to git2 just for this — the HTTP path covers 99% of cases.
- Promote `crud-core` to be async-friendly by using `tokio::task::spawn_blocking` around its sync API in the MCP layer.
- Do NOT make `crud-core` itself async — keeps CLI binary lean and reuses the same battle-tested sync code.
- `ignore::WalkBuilder::threads(num_cpus)` enables parallel walk. Free upgrade — same crate.
- Handlebars-rust supports custom helpers (`handlebars_helper!` macro). Register `{{eq}}`, `{{add}}`, etc. helpers — keeps `.hbs` extension and PRD compatibility.
## Version Compatibility
| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `clap` 4.6 | `rustc` ≥ 1.74 | Pin `rust-version = "1.75"` in workspace to be safe. |
| `thiserror` 2.x | `rustc` ≥ 1.61 | 2.x is breaking vs 1.x: error display attributes changed slightly. Greenfield = use 2.x. |
| `handlebars` 6.x | `serde` 1.x | 6.x added stricter strict-mode by default — opt out with `.set_strict_mode(false)` if needed for missing-var tolerance during dev. |
| `ureq` 3.x | rustls (default) | Default TLS is rustls, no OpenSSL dependency — preserves single-binary story. |
| `dirs` 6.x | — | 6.x dropped some legacy fallbacks. For `~/.crud/`, just use `dirs::home_dir()` and append `.crud`. |
| `toml` 1.x | `serde` 1.x | Stable; round-trips via `toml_edit` (re-exported as `toml::Value`'s edit-preserving twin). |
| `inquire` 0.9 | `crossterm` 0.27 (default) | Default backend; no extra setup. |
| `insta` 1.47 | — | Install `cargo install cargo-insta` for the `review` workflow. |
## Confidence Levels
| Recommendation | Confidence | Rationale |
|----------------|------------|-----------|
| `clap` derive | HIGH | No real alternative. Verified current version 4.6.1 on crates.io. |
| `handlebars` 6.4 | HIGH | PRD explicitly requires `.hbs`/Handlebars syntax — this is forced. Version verified. |
| `serde` + `toml` + `serde_json` (skip figment) | HIGH | Single-file config doesn't need merge engine. |
| `ignore` over `walkdir` | HIGH | `.crudignore` is a near-certain future request; cheap to adopt now. |
| `ureq` + tarball over `git2`/`gix` | HIGH | Cross-checked: PRD wants single binary, git2 = C deps, gix = compile bloat, codeload URL is documented and stable. |
| `inquire` over `dialoguer` | MEDIUM | Both are fine. Inquire's prompt variety and validator macros tip the scale for `setup`. Either is defensible. |
| 2-crate workspace | HIGH | PRD's "core/cli separation for future MCP" demands compiler-enforced boundary. |
| `tracing` over `log`+`env_logger` | MEDIUM-HIGH | Slight over-spec for v1, but free insurance against MCP-server migration. |
| `insta` + `assert_cmd` testing | HIGH | Industry standard for code-generator + CLI testing respectively. |
## Sources
- crates.io API (live, fetched 2026-05-27) — version numbers for all 32 crates listed
- [clap official derive tutorial](https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html) — derive API is the recommended 2026 path
- [Rain's Rust CLI recommendations](https://rust-cli-recommendations.sunshowers.io/handling-arguments.html) — top-level struct + flatten + global=true pattern
- [askama-rs template benchmark](https://github.com/askama-rs/template-benchmark) — handlebars vs minijinja vs tera vs askama performance
- [minijinja benchmarks](https://github.com/mitsuhiko/minijinja/tree/main/benchmarks) — concrete compile/render numbers cited
- [LogRocket: Top 3 templating libraries for Rust](https://blog.logrocket.com/top-3-templating-libraries-for-rust/) — ecosystem positioning
- [gitoxide discussion #1381](https://github.com/GitoxideLabs/gitoxide/discussions/1381) — confirms gix is not yet a drop-in for git2 for many workflows
- [GitHub codeload URL pattern](https://docs.github.com/en/repositories/working-with-files/using-files/downloading-source-code-archives) — `codeload.github.com/{user}/{repo}/tar.gz/{ref}` is the stable, no-auth path
- [fadeevab — Comparison of Rust CLI prompts](https://fadeevab.com/comparison-of-rust-cli-prompts/) — dialoguer vs inquire vs cliclack vs promptly side-by-side
<!-- GSD:stack-end -->

<!-- GSD:conventions-start source:CONVENTIONS.md -->
## Conventions

Conventions not yet established. Will populate as patterns emerge during development.
<!-- GSD:conventions-end -->

<!-- GSD:architecture-start source:ARCHITECTURE.md -->
## Architecture

Architecture not yet mapped. Follow existing patterns found in the codebase.
<!-- GSD:architecture-end -->

<!-- GSD:skills-start source:skills/ -->
## Project Skills

No project skills found. Add skills to any of: `.claude/skills/`, `.agents/skills/`, `.cursor/skills/`, `.github/skills/`, or `.codex/skills/` with a `SKILL.md` index file.
<!-- GSD:skills-end -->

<!-- GSD:workflow-start source:GSD defaults -->
## GSD Workflow Enforcement

Before using Edit, Write, or other file-changing tools, start work through a GSD command so planning artifacts and execution context stay in sync.

Use these entry points:
- `/gsd-quick` for small fixes, doc updates, and ad-hoc tasks
- `/gsd-debug` for investigation and bug fixing
- `/gsd-execute-phase` for planned phase work

Do not make direct repo edits outside a GSD workflow unless the user explicitly asks to bypass it.
<!-- GSD:workflow-end -->



<!-- GSD:profile-start -->
## Developer Profile

> Profile not yet configured. Run `/gsd-profile-user` to generate your developer profile.
> This section is managed by `generate-claude-profile` -- do not edit manually.
<!-- GSD:profile-end -->
