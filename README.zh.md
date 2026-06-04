# crud-cli

**语言：** [English](./README.md) · 简体中文

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

一个用 Rust 编写的命令行工具，配合 AI 编程 Agent（Claude Code、Cline、Copilot 等）生成后台管理系统的 CRUD 脚手架。

AI 调用的 Token 成本主要来自**输出 Token**——Agent 每次把完整的模板代码写出来，输出量巨大，费用和等待时间都很高。`crud-cli` 的思路是把模板存在本地，Agent 只下发一条短命令 + 结构化数据，由 CLI 在本地完成渲染，从而大幅降低输出成本、加快生成速度，同时保证产物与项目既有代码风格逐字节一致。

## Token 开销参考

下表使用 [tiktoken](https://github.com/openai/tiktoken) 对两个场景的渲染产物做了粗略估算，对比 `crud-cli gen` 生成的完整代码（native）与触发生成所需的结构化指令（json）的 Token 数量。示例数据来自 [crud-templates](https://github.com/ttTennessee/crud-templates) 仓库中的 `_example_sub.json` 和 `_example_tree.json`。

| 场景 | 生成代码量 | 输入指令量 | 差值 |
|------|----------|---------|------|
| 主子表（sub） | 18,325 | 1,151 | ~94% |
| 树形结构（tree） | 18,166 | 689 | ~96% |

> **说明：** 这里统计的是生成产物与输入指令的 Token 体量差异，并非真实节省量——实际调用中还有系统提示、上下文、MCP 工具调用等开销，真实节省比例因场景而异，以上数据仅供量级参考。

**适合你吗？**

- 如果你的项目以**标准 CRUD 页面**为主（列表、表单、详情、导入导出），这个工具非常适合——重复的模板代码正是 Agent 最浪费 Token 的地方。
- 如果业务逻辑复杂、每张表都有大量定制代码，模板复用率低，收益会相对有限，不一定值得引入当前工具。

## 安装

从 [Releases](https://github.com/ttTennessee/crud-cli/releases) 下载预编译二进制（含 MCP server）：

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

或从源码构建（较新版本的 Rust stable 工具链；项目未承诺 MSRV）：

```bash
cargo build --release --features full   # CLI + MCP server
```

完整安装说明：[docs/zh-CN/quickstart.md](docs/zh-CN/quickstart.md#安装)。

## Hello world

```bash
# 1. 初始化项目配置
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views

# 2. 安装现成模板包（或自己在 .crud/templates/ 下写模板）
crud-cli template install

# 3. 生成代码
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

完整端到端走查（含原理说明）：[docs/zh-CN/quickstart.md](docs/zh-CN/quickstart.md)。

## 默认模板仓库

[crud-templates](https://github.com/ttTennessee/crud-templates) 是配套的默认模板仓库，收录可直接通过 `crud-cli template install` 安装的现成 CRUD 模板包。中后台框架生态庞杂，仅靠个人维护难以覆盖，欢迎一起来贡献新模板。

## 文档

| 主题 | 链接 |
|---|---|
| 安装、基本使用、CLI 子命令参考 | [docs/zh-CN/quickstart.md](docs/zh-CN/quickstart.md) |
| 模板结构与编写指南 | [docs/zh-CN/templates.md](docs/zh-CN/templates.md) |
| MCP server——配置、工具、资源 | [docs/zh-CN/mcp.md](docs/zh-CN/mcp.md) |
| `entity.json` 规范 | [docs/zh-CN/entity.md](docs/zh-CN/entity.md) |
| MCP 提供的模板编写 spec（英文唯一） | [agent-resources/template-authoring.md](agent-resources/template-authoring.md) |
| 贡献者文档（英文） | [docs/dev/](docs/dev/) |

## 架构

单 crate、三层模块，上层两层用 Cargo feature 开关，使用方按需付出依赖代价。

- `src/core/` —— 纯逻辑：配置、路径、模板引擎、事务写盘、validator、类型化错误。不依赖 clap / inquire / tokio。
- `src/cli/` —— feature `cli`（默认）。`clap` 命令行、`inquire` 向导、Agent JSON 输出、人类可读输出。
- `src/mcp/` —— feature `mcp`。基于 `rmcp` + `tokio` 的 MCP server（`crud-cli mcp`），通过 stdio 复用 `core`。

Feature 组合：`default = ["cli"]`、`cli`、`mcp`、`full = ["cli", "mcp"]`。使用 `--no-default-features` 可把 `crud_cli` 作为纯库依赖。

## 许可证

MIT，详见 [LICENSE](./LICENSE)。Copyright (c) 2026 Yujie Jin.
