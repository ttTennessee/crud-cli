## Project

**crud-cli**

`crud-cli` 是一个用 Rust 编写的命令行工具，用于配合 AI Agent 高效生成后台管理系统的 CRUD 代码。通过把重复的模板本地化，让 Agent 只下发"命令 + 结构化数据"，由本地 CLI 渲染并落盘，从而把 Agent 的 Token 消耗降低约 40 倍（从 2000+ 降到 ~50）。面向使用 AI Agent（Cline、Copilot、Claude Code 等）进行后台管理系统开发的工程师。

**Core Value:** 让 AI Agent 用一条几十字符的命令，瞬间在本地渲染出与项目代码风格一致的全套 CRUD 文件（前后端），彻底消除 Agent 输出模板代码带来的 Token 浪费和风格漂移。

### Code Search Tools
MCP tool usage:
- `semble`: hybrid semantic + lexical search for code snippets. Use when the user asks a "what" or "how" question about code.
- `codegraph`: knowledge graph of codebase structure, call graphs, and dependencies. Use when the user asks about relationships, callers, callees, or impact analysis.

### Constraints

- **技术栈**：Rust（`cargo` 构建），单二进制分发 — 来自 PRD 设计
- **架构**：`core` 与 `cli` 严格分离，为后续 MCP Server 预留接口 — 来自 PRD 设计决策
- **兼容性**：项目模板路径固定为 `.crud/templates/`，全局模板路径固定为 `~/.crud/templates/<模板名>/`
- **配置**：项目配置文件位于 `<项目根>/.crud/setup.toml`（TOML 格式）
- **安全**：默认不覆盖已有文件 — 防止 Agent 误操作覆盖用户代码
- **模板引擎**：PRD 示例使用 `.hbs` 后缀，倾向 Handlebars 风格语法

## Technology Stack

> **Status (2026-06-04):** This document mixes the *original pre-build
> recommendation* (the "Recommended Stack" / "Alternatives" / "Confidence
> Levels" sections below) with *as-built reality* (the "TL;DR — As Built",
> "Architecture", and "Conventions" sections). Where they conflict, **trust
> `Cargo.toml` and the Architecture section**. Items in the recommendation
> tables that have since diverged are marked ⚠️.
>
> Major divergences from the original plan:
> 1. **Single crate, not 2-crate workspace.** Boundary enforced by a `cli`
>    Cargo feature instead of a separate `crud-core` crate.
> 2. **MCP server has shipped.** `src/mcp/` (gated by the `mcp` feature, with
>    `full = ["cli", "mcp"]`) is no longer "deferred to v2."
> 3. **`tokio` is in the dependency graph** (required by `rmcp`), under the
>    `mcp` feature only. The CLI binary built without `mcp` is still sync.
> 4. **`insta` / `assert_cmd` / `predicates` were not adopted**; integration
>    tests use plain `std::process::Command` + `tempfile`.
> 5. **`cargo-dist` is adopted** (see `dist-workspace.toml` and
>    `[profile.dist]` in `Cargo.toml`), not "optional / once 1.0 ships."

## TL;DR — As Built
- **Layout:** single crate `crud-cli` with `src/{core,cli,mcp}/` modules. `core` is pure logic (no clap, no inquire, no tokio). The `cli` Cargo feature pulls in `clap` + `inquire` + `tracing-subscriber`; the `mcp` feature pulls in `rmcp` + `tokio` + `schemars`. `full = ["cli", "mcp"]` builds the combined binary that exposes `crud-cli mcp`.
- **Features:** `default = ["cli"]`, `cli`, `mcp`, `full`. The binary requires `cli`; the library (`crud_cli`) can be consumed with `--no-default-features` for embedding.
- **CLI:** `clap` 4.6 (derive).
- **Templates:** `handlebars` 6.0 — PRD specifies `.hbs` syntax.
- **Config:** `serde` + `toml` 0.8 + `serde_json`. Single-file `.crud/setup.toml`; no merge engine.
- **Frontmatter:** `gray_matter` 0.2 — template files carry YAML frontmatter parsed before render.
- **Filesystem walk:** `ignore` 0.4 (the ripgrep crate).
- **GitHub fetching:** `ureq` 3 (rustls only, no native-tls) + `flate2` + `tar` against `https://codeload.github.com/{user}/{repo}/tar.gz/HEAD`. `sha2` for integrity checksums on installed bundles.
- **MCP server:** `rmcp` 1.7 + `tokio` 1.52 (multi-thread) + `schemars` 1. Gated by `mcp` feature.
- **Errors:** `thiserror` 2 in `core` (typed `ErrorEnvelope`); `anyhow` in the CLI surface for top-level `?` ergonomics.
- **Prompts:** `inquire` 0.9.
- **Time:** `time` 0.3 (used for install timestamps / metadata).
- **Fuzzy matching:** `strsim` 0.11 (used by validator for "did you mean …" suggestions).
- **Logging:** `tracing` + `tracing-subscriber` (subscriber gated behind `cli`).
- **Testing:** plain `std::process::Command` + `tempfile`; tests live under `tests/` with the agent-facing contract surface in `tests/contracts/`. No `insta`, no `assert_cmd`.
- **Release:** `cargo-dist` (`dist-workspace.toml`, `[profile.dist]` with `lto = "thin"`).

## Recommended Stack
*(Original pre-build recommendation. Cross-check with TL;DR — As Built; ⚠️ marks items that diverged.)*
### Core Technologies
| Technology | Version | Purpose | Why Recommended |
|------------|---------|---------|-----------------|
| `clap` (derive) | 4.6.1 | CLI parsing, subcommands, help, completion | De-facto standard. Powers `cargo`, `ripgrep`, `bat`, `fd`. Derive API is the recommended path in 2025/2026 per official tutorial. No serious competitor for a multi-subcommand CLI of this shape. |
| `handlebars` | 6.4.1 | Template rendering of `.hbs` files | **PRD explicitly specifies Handlebars syntax** (`{{model}}Controller.java.hbs`). Mature, used by rust-lang.org itself, supports custom helpers (needed for `snake_case`/`PascalCase`/`camelCase` filters). |
| `serde` + `serde_derive` | 1.0.228 | Universal (de)serialization | Required by every config/data crate below. |
| `toml` | 0.8 ⚠️ | Read/write `.crud/setup.toml` | Actual pin is `toml = "0.8"` — pre-1.0 line. The 1.x suggestion was forward-looking; 0.8 covers our needs. |
| `serde_json` | 1.0.150 | Read `gen --file <json>` input | Standard. |
| `anyhow` | 1.0.102 | Error type in `crud-cli` (binary boundary) | Standard pairing: `anyhow` for "I just want a `?` with context at the top of the binary", `thiserror` for typed library errors. |
| `thiserror` | 2.0 | Typed errors in `core` | The 2.x line landed in late 2024; stable. Lets `core` expose enum-shaped errors that CLI and MCP layers pattern-match. |
### Supporting Libraries
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `inquire` | 0.9.4 | Interactive `crud-cli setup` prompts (Select / MultiSelect / Confirm / Text) | Use for all interactive prompts. Richer than `dialoguer`, better validators, derive macros for enums. |
| `ignore` | 0.4.25 | Walk `.crud/templates/` and `~/.crud/templates/<name>/` | Use instead of `walkdir`. Free `.gitignore`/`.crudignore` support; parallel walker; battle-tested by ripgrep. |
| `ureq` | 3.0 | Blocking HTTP GET of GitHub tarballs | Use for `template install`. Configured with `default-features = false, features = ["rustls"]` — no native-tls, no tokio. |
| `flate2` | 1.0 | Gunzip tarball streams | Pair with `tar`. Pure-Rust `miniz_oxide` backend (no zlib C dep). |
| `tar` | 0.4 | Untar tarball into `~/.crud/templates/<name>/` | Standard. Pair with `flate2::read::GzDecoder`. |
| `tempfile` | 3.14 | Atomic writes during template install & tests | Write tarball to temp dir, untar, then `rename` into final location. |
| `dirs` | 6.0.0 | Resolve `~/.crud/` cross-platform | Use `dirs::home_dir()`. Simpler than `directories` (which encodes XDG vs macOS vs Windows conventions — we deliberately want `~/.crud/` on all platforms per PRD). |
| `tracing` | 0.1 | Structured logging in `core` | Use `tracing::{info, debug, warn, error}` in `core`. CLI and MCP layers each install their own subscriber. |
| `tracing-subscriber` | 0.3 | Logging output (gated by `cli` feature) | Wire to stderr; respects `--verbose` flag. |
| `serde_path_to_error` | 0.1.x | Better JSON/TOML deserialization errors | Optional but cheap — turns `expected string, found null` into `at .fields[0].type: expected string`. Huge UX win for `--file <json>` and `validate`. |
| `globset` | 0.4 | Match template file globs | Directly declared (not re-exported via `ignore`). Used by template discovery & overwrite-policy filtering. |
| `convert_case` | 0.6 | Identifier case conversion | Helpers (`to_case(Case::Snake)`, etc.) for Handlebars `{{snake model}}` etc. |
| `gray_matter` | 0.2 | YAML frontmatter parsing for templates | ⚠️ Not in original recommendation. Templates carry frontmatter (`outputs:`, `when:`, etc.) parsed before render. |
| `strsim` | 0.11 | Fuzzy "did you mean …" suggestions | ⚠️ Not in original recommendation. Used by validator and CLI argument errors. |
| `sha2` | 0.10 | Bundle integrity hashes | ⚠️ Not in original recommendation. Installed template bundles record SHA-256 in metadata. |
| `time` | 0.3 | Local-time timestamps | ⚠️ Not in original recommendation. Used for install / metadata timestamps; features = `formatting, macros, local-offset`. |
| `rmcp` | 1.7 | MCP server (gated by `mcp` feature) | ⚠️ Not in original recommendation — original plan deferred MCP. Features: `server, transport-io, macros, schemars`. |
| `tokio` | 1.52 | Async runtime for `rmcp` (gated by `mcp` feature) | ⚠️ Original plan said "no async runtime in v1." Adopted only under the `mcp` feature; the default `cli`-only binary remains sync. |
| `schemars` | 1 | JSON Schema generation for MCP tool inputs (gated by `mcp` feature) | ⚠️ Not in original recommendation. |
### Development Tools
| Tool | Status | Notes |
|------|--------|-------|
| `tempfile` 3.14 | **Adopted** | Per-test sandbox dirs (`tempdir()`); also used at runtime for atomic installs. |
| `cargo-dist` | **Adopted** | See `dist-workspace.toml` and `[profile.dist] inherits = "release", lto = "thin"`. Drives the cross-platform release pipeline (`release.yml`). |
| `insta` | ⚠️ **NOT adopted** | Original plan called for snapshot testing of rendered templates. Current tests assert rendered output via direct string comparison. Reconsider if template regressions become a pain point. |
| `assert_cmd` + `predicates` | ⚠️ **NOT adopted** | CLI tests use plain `std::process::Command` (see `tests/contracts/*`). |
| `cargo-nextest` | Not adopted | Still recommended for local dev; CI uses `cargo test`. |
## Layout Decision (historical)
### Original tradeoff table — for context only
| Option | Original verdict | Actual outcome |
|--------|------------------|----------------|
| Single crate with `cli/` + `core/` modules | **Rejected** ("nothing enforces the boundary") | ✅ **Adopted** — boundary enforced by the `cli` Cargo feature instead of a crate split. `core` cannot import clap/inquire because they aren't compiled when the feature is off. |
| 2-crate workspace (`crud-core` + `crud-cli`) | **Adopted** | ❌ Not adopted. The feature-gate approach achieved the same compiler-enforced boundary with less ceremony. |
| 3-crate today (add empty `crud-mcp` stub) | **Rejected for v1** | ❌ MCP shipped as `src/mcp/` in-tree, gated by the `mcp` feature. No separate crate. |
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
| Async runtime in the default (`cli`) binary | Adds ~50 deps, slows CLI startup, complicates error handling. CLI work is sync; tarball download uses blocking `ureq`. | Blocking `ureq`. ⚠️ Update: `tokio` IS pulled in under the `mcp` feature because `rmcp` requires it — but the `cli`-only binary remains fully sync. |
| `serde_yaml` | Unmaintained as of 2024. | If YAML ever needed: `serde_yml` (community fork) or `serde_yaml_ng`. Not needed for v1. |
| `prettytable-rs` for `template list` | Unmaintained. | `comfy-table` or just `println!` formatted output. |
## Stack Patterns (as built + future variants)
- **Template install over HTTP:** `ureq` + `flate2` + `tar` against `https://codeload.github.com/{user}/{repo}/tar.gz/{ref}`. `GITHUB_TOKEN`/`GH_TOKEN` env vars are not yet wired up — add `Authorization: Bearer …` when private-repo support is needed.
- **MCP server (as built):** `src/mcp/server.rs` builds an `rmcp` server on top of `tokio` (multi-thread) and reuses `core` sync APIs directly. No `spawn_blocking` shim today; the sync work (template render, fs writes) is short enough that blocking the tokio runtime hasn't caused issues. Revisit with `spawn_blocking` if MCP handlers ever touch large directory walks.
- **Do NOT make `core` itself async** — keeps the CLI binary lean and lets MCP reuse the same sync code path.
- `ignore::WalkBuilder::threads(num_cpus)` enables parallel walk if directory size grows. Free upgrade — same crate.
- Handlebars custom helpers (`handlebars_helper!` macro) are registered for `{{snake}}`, `{{pascal}}`, `{{camel}}` etc. via `convert_case`. Add new ones in `src/core/template_engine.rs`.
## Version Compatibility
| Package A | Compatible With | Notes |
|-----------|-----------------|-------|
| `clap` 4.6 | `rustc` ≥ 1.74 | Track stable; no `rust-version` pin (project does not commit to an MSRV). |
| `thiserror` 2.x | `rustc` ≥ 1.61 | 2.x is breaking vs 1.x: error display attributes changed slightly. Greenfield = use 2.x. |
| `handlebars` 6.x | `serde` 1.x | 6.x added stricter strict-mode by default — opt out with `.set_strict_mode(false)` if needed for missing-var tolerance during dev. |
| `ureq` 3.x | rustls (default) | Default TLS is rustls, no OpenSSL dependency — preserves single-binary story. |
| `dirs` 6.x | — | 6.x dropped some legacy fallbacks. For `~/.crud/`, just use `dirs::home_dir()` and append `.crud`. |
| `toml` 0.8 | `serde` 1.x | ⚠️ Actual pin. Edit-preserving round-trip via `toml_edit` if/when needed. |
| `inquire` 0.9 | `crossterm` 0.27 (default) | Default backend; no extra setup. |
| `rmcp` 1.7 | `tokio` 1.x | Required for `mcp` feature. Pulls in `tokio-util`, `bytes`, etc. Hard MCP dependency; transports configured via `transport-io` (stdio). |
## Confidence Levels (original recommendation — kept for context)
| Recommendation | Original confidence | As-built status |
|----------------|---------------------|-----------------|
| `clap` derive | HIGH | ✅ As planned. |
| `handlebars` 6.x | HIGH | ✅ As planned. |
| `serde` + `toml` + `serde_json` (skip figment) | HIGH | ✅ As planned (pin is `toml 0.8`, not 1.x). |
| `ignore` over `walkdir` | HIGH | ✅ As planned. |
| `ureq` + tarball over `git2`/`gix` | HIGH | ✅ As planned. |
| `inquire` over `dialoguer` | MEDIUM | ✅ As planned. |
| 2-crate workspace | HIGH | ❌ Replaced by single crate + `cli`/`mcp` Cargo features. |
| `tracing` over `log`+`env_logger` | MEDIUM-HIGH | ✅ As planned. |
| `insta` + `assert_cmd` testing | HIGH | ❌ Not adopted — see Development Tools table. |
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

## Conventions

- **Errors:** `core` returns typed `ErrorEnvelope` (`src/core/error.rs`) with a
  `Kind`, an exit code, a human hint, and a JSON `details` map. The CLI converts
  these to either human output or, under `--agent`, a single JSON object on stderr.
- **i18n:** user-facing strings are keys in `src/core/i18n/keys.rs`, rendered via
  `i18n::t` / `i18n::tf`. Don't hardcode user-facing English in command code.
- **Config schemas** are `#[serde(deny_unknown_fields)]` — typos surface as errors.
  Section order in `setup.toml` is part of the contract (round-tripped on write).
- **Frontmatter:** template files carry YAML frontmatter parsed via `gray_matter`
  (see `src/core/template_meta.rs`). Frontmatter drives `outputs:` (per-file output
  paths), `when:` (conditional render), and other per-template behavior.
- **Tests:** plain `std::process::Command` + `tempfile`; `tests/contracts/` locks the
  agent-facing surface (panic discipline, agent JSON output, byte-identical render).
  No `insta`, no `assert_cmd` today — see "Development Tools" if reconsidering.
- **Lint policy:** `unwrap_used` / `expect_used` / `panic` are denied in
  `[lints.clippy]` — propagate errors via `ErrorEnvelope`/`Result` instead.
- **Stream discipline:** `println!`/`eprintln!` are only allowed in
  `src/cli/output.rs`; CI greps for violations elsewhere.

## Architecture

Single crate `crud-cli` with three module layers and feature-gated upper layers:

- `default = ["cli"]` — the standard binary.
- `cli` — pulls in `clap`, `inquire`, `tracing-subscriber`. Required by the binary.
- `mcp` — pulls in `rmcp`, `tokio`, `schemars`. Adds the `crud-cli mcp` subcommand.
- `full = ["cli", "mcp"]` — combined binary.

Library consumers can depend on `crud_cli` with `--no-default-features` for pure
`core` usage (no async, no CLI deps).

### `src/core/` — pure logic (no clap, no inquire, no tokio)
- **Config:** `config.rs` (project `setup.toml`), `global_config.rs` (`~/.crud/config.toml`).
- **Paths:** `paths.rs`, `default_paths.rs`, prefix rebasing inside `gen_pipeline.rs`.
- **Template engine + loading:** `template_engine.rs`, `template_loader.rs`,
  `template_meta.rs` (frontmatter), `template_variables.rs` (per-call variable schema).
- **Generation pipeline:** `gen_pipeline.rs` orchestrates; `gen_input.rs` (parse
  user input), `gen_context.rs` (build the Handlebars context), `gen_run.rs`
  (run parameters), `gen_report.rs` (post-run report for agent JSON).
- **Field DSL:** `field_dsl.rs` parses the compact `name:type` shorthand;
  `field_types.rs` defines the type catalogue; `type_map.rs` maps types per backend.
- **Filesystem:** `fs_writer.rs` (transactional / batch-atomic writes).
- **Validation:** `validator.rs` (frontmatter + render dry-run + unknown-var check).
- **Global templates:** `template_installer.rs`, `template_install_meta.rs`,
  `template_meta_global.rs`.
- **Misc:** `git_info.rs` (commit metadata for generated headers), `i18n/`,
  `error.rs` (typed `thiserror` enums + `ErrorEnvelope`).

### `src/cli/` — clap surface (feature-gated on `cli`)
- `args.rs` (clap definitions), `setup_wizard.rs` (inquire flow),
  `commands/{setup,gen,validate,template,mcp}.rs` (subcommand handlers),
  `agent_mode.rs` (single-line JSON to stderr under `--agent`),
  `output.rs` (only place allowed to use `println!`/`eprintln!`).
- Depends on `core`; never the reverse.

### `src/mcp/` — MCP server (feature-gated on `mcp`)
- `server.rs` builds the `rmcp` server over stdio transport.
- `context.rs` carries the project root and shared state.
- `convert.rs` translates `ErrorEnvelope` ↔ MCP error responses.
- `resources.rs` exposes MCP resources (template metadata etc.).
- `validate_logic.rs` is the MCP-side validate handler — reuses `core::validator`.
- Depends on `core` only; does **not** depend on `cli`.

**Paths model:** template subdirectory prefixes (the first path segment, e.g.
`java/`, `vue/`, `resources/`, `doc/`) are looked up in `[paths.lang]` first, then
`[paths.aux]`, and rebased to the configured project directory. The model is
language-based and open-ended — there is no fixed list of framework prefixes.

**Global templates:** `crud-cli template install` downloads a template bundle from a
GitHub repo into `~/.crud/templates/<name>/<version>/`; `template use` points a
project's `[project].template` at one; `template list` enumerates installed bundles.
The default repo is configurable via `[templates].repo` in `~/.crud/config.toml`.
