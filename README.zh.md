# crud-cli

一个用 Rust 编写的命令行工具，配合 AI 编程 Agent（Claude Code、Cline、Copilot 等）
生成后台管理系统的 CRUD 脚手架，**避免** Agent 在模板样板代码上浪费 Token。

Agent 只下发一条短命令 + 结构化数据，`crud-cli` 在本地完成模板渲染。
单次 CRUD 的 Token 成本可从 2000+ 降到 ~50，约 **40 倍** 缩减，
同时保证产物与项目既有代码风格逐字节一致。

> English: [README.md](./README.md)

## 当前状态

早期开发中。已实现：

- `crud-cli setup` — 交互式向导 + 非交互式 flag 模式，写入 `.crud/setup.toml`
  （backend / frontend / component-library / overwrite-policy 四个维度）。
- 两阶段 plan + commit 事务式文件写盘，失败时不留半成品。
- Agent 模式（`--agent`）：错误以结构化 JSON envelope 输出到 stderr，
  成功时 stdout 为空 — 便于 LLM 解析。
- Handlebars 模板引擎接入（渲染管线就绪，面向用户的 `gen` 命令在后续 phase 落地）。

尚未实现：`gen`、`template install`、`validate`、`template list`。
完整目标见 PRD（`prd.html`）。

## 安装

需要 Rust ≥ 1.75。

```bash
git clone <this repo>
cd crud-cli
cargo build --release
# 二进制位于 ./target/release/crud-cli
```

`cargo install` / 预编译二进制会在 v0.1 发布时提供。

## 快速开始

### 交互式 setup

```bash
crud-cli setup
```

四个选项交互式选择完毕后，在当前项目根目录写入 `.crud/setup.toml`。

### 非交互式 setup（Agent / CI 用）

任一 flag 传入时，四个 flag 全部必填：

```bash
crud-cli setup \
  --backend spring-boot \
  --frontend vue \
  --component-library element-plus \
  --overwrite-policy never
```

可选值：

| Flag | 取值 |
|---|---|
| `--backend` | `spring-boot`, `nest`, `none` |
| `--frontend` | `vue`, `react`, `none` |
| `--component-library` | `element-plus`, `antd`, `naive-ui`, `none` |
| `--overwrite-policy` | `never`, `force-only`, `always` |
| `--force` | 仅在 `overwrite-policy=force-only` 时生效 |

### Agent 模式

```bash
crud-cli --agent setup --backend nest --frontend react \
  --component-library antd --overwrite-policy never
```

- 成功：exit 0，stdout 空。
- 失败：exit 非 0，stderr 输出单个 JSON 对象，包含
  `code`、`message`、`flag`、`value`、`remediation`，可直接回喂给模型。

## 配置

项目配置文件位于 `<项目根>/.crud/setup.toml`。模板查找顺序：

1. `<项目根>/.crud/templates/`（项目级，优先）
2. `~/.crud/templates/<模板名>/`（用户级）

TOML schema 是锁定的 — 未知字段会被拒绝，避免 Agent 漂写文件结构。

## 架构

`core` 与 `cli` 严格分层，为未来的 MCP Server 复用预留接口：

- `src/core/` — 纯逻辑：配置解析、路径解析、模板引擎、事务写盘、
  `thiserror` 类型化错误。不依赖 clap / inquire。
- `src/cli/` — `clap` 命令行、`inquire` 向导、Agent JSON 输出、
  人类可读输出。依赖 `core`，反向永不依赖。

`cli` feature 可关闭（`--no-default-features`），从而把 `crud_cli` 作为
纯库依赖，不引入 clap/inquire。

## 测试

```bash
cargo test            # 单元 + 集成 + 契约测试
```

契约测试（`tests/contracts/`）锁定 Agent 面向的接口：panic 行为、
JSON 错误 envelope 形状、setup 写盘的字节一致性。

## 许可证

MIT OR Apache-2.0
