# crud-cli

**语言：** [English](./README.md) · 简体中文

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

一个用 Rust 编写的命令行工具，配合 AI 编程 Agent（Claude Code、Cline、Copilot 等）
生成后台管理系统的 CRUD 脚手架。

AI 调用的 Token 成本主要来自**输出 Token**——Agent 每次把完整的模板代码写出来，
输出量巨大，费用和等待时间都很高。`crud-cli` 的思路是把模板存在本地，
Agent 只下发一条短命令 + 结构化数据，由 CLI 在本地完成渲染，从而大幅降低输出成本、
加快生成速度，同时保证产物与项目既有代码风格逐字节一致。

## Token 开销参考

下表使用 [tiktoken](https://github.com/openai/tiktoken) 对两个场景的渲染产物做了粗略估算，
对比 `crud-cli gen` 生成的完整代码（native）与触发生成所需的结构化指令（json）的 Token 数量。
示例数据来自 [crud-templates](https://github.com/ttTennessee/crud-templates) 仓库中的 `_example_sub.json` 和 `_example_tree.json`。

| 场景 | 生成代码量 | 输入指令量 | 差值 |
|------|----------|---------|------|
| 主子表（sub） | 18,325 | 1,151 | ~94% |
| 树形结构（tree） | 18,166 | 689 | ~96% |

> **说明：** 这里统计的是生成产物与输入指令的 Token 体量差异，并非真实节省量——
> 实际调用中还有系统提示、上下文、MCP 工具调用等开销，真实节省比例因场景而异，
> 以上数据仅供量级参考。

**适合你吗？**

- 如果你的项目以**标准 CRUD 页面**为主（列表、表单、详情、导入导出），
  这个工具非常适合——重复的模板代码正是 Agent 最浪费 Token 的地方。
- 如果业务逻辑复杂、每张表都有大量定制代码，模板复用率低，
  收益会相对有限，不一定值得引入当前工具。

## 默认模板仓库

[crud-templates](https://github.com/ttTennessee/crud-templates) 是配套的默认模板仓库，
提供开箱即用的 Java + Vue CRUD 模板集，可通过 `crud-cli template install` 直接安装。

## 安装

### 预编译二进制（推荐）

前往 [Releases](https://github.com/ttTennessee/crud-cli/releases) 下载对应平台的压缩包，
解压后将 `crud-cli`（Windows 为 `crud-cli.exe`）放到任意 PATH 目录即可。

或使用一键安装脚本：

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

### 从源码构建

需要较新版本的 Rust stable 工具链（项目未承诺 MSRV）。

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release                 # 仅 CLI
cargo build --release --features full # CLI + MCP server
# 二进制位于 ./target/release/crud-cli
```

## 快速开始

### 1. Setup

项目配置（checked in，团队共享）。`--backend` / `--frontend` 接收语言标识，
`--lang` / `--aux` 设置路径映射：

```bash
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views \
  --aux resources=src/main/resources --aux doc=doc/api
```

用户配置（每人一份，gitignored）：

```bash
crud-cli setup --user-name "Alice" --user-email alice@example.com \
  --overwrite-policy force-only --enabled-types backend
```

去掉 `--project` 或用户相关 flag，会进入交互式向导。

### 2. 放一个模板

`.crud/templates/java/Controller.java.hbs`：

```handlebars
---
basePath: "java/{{package_path}}/controller"
filename: "{{model_pascal}}Controller.java"
---
package {{package}}.controller;

@RestController
@RequestMapping("/{{model_kebab}}")
public class {{model_pascal}}Controller {
    // ...
}
```

### 3. 生成

```bash
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

输出落到 `<paths.lang.java>/com/acme/demo/controller/UserController.java`。

### 4. 提交前校验

```bash
crud-cli validate
# validate ok: 1 templates
```

## 路径系统

模板本身是项目无关的；具体落到哪里由 `.crud/setup.toml` 里的两张路径表决定。
模板位置的第一段路径（**前缀**）会先在 `[paths.lang]` 查找，找不到再到
`[paths.aux]`，然后把该前缀替换成配置的真实目录。这套模型是按语言组织、
开放式的 —— 没有固定的框架前缀列表；你往 `[paths.lang]` / `[paths.aux]`
里加的任何 key 都能当前缀用。

`setup` 按所选语言种入的约定默认值：

| 前缀 | 所在表 | 默认值 | 适用于 |
|---|---|---|---|
| `java` | `[paths.lang]` | `src/main/java` | backend = java |
| `ts` | `[paths.lang]` | `src` | backend = typescript |
| `go` | `[paths.lang]` | `internal` | backend = go |
| `python` | `[paths.lang]` | `src` | backend = python |
| `vue` | `[paths.lang]` | `src/views` | frontend = vue |
| `react` | `[paths.lang]` | `src/views` | frontend = react |
| `resources` | `[paths.aux]` | `src/main/resources` | backend = java |
| `doc` | `[paths.aux]` | `doc/api` | 多数后端 |

多模块/monorepo 项目，按布局覆写：

```toml
[paths.lang]
java = "backend/api/src/main/java"
vue = "frontend/src/views"

[paths.aux]
resources = "backend/api/src/main/resources"
doc = "docs/api"
```

`.crud/templates/java/Foo.hbs`（或在 front-matter 写
`basePath: "java/{{package_path}}/foo"`），不管宿主项目布局如何，
都会落到配置的 `java` 路径下。

## 模板编写

### Front-matter

任意 `.hbs` 文件顶部的可选 YAML 块：

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only          # never | force-only | always
---
```

`basePath` 里可以引用任何内置变量或 schema 声明的变量。`filename` 必须是
单段（不能含 `/`）。

**条件渲染** —— `generateWhen` / `skipWhen` 控制这个文件是否生成（二者互斥，
同时出现会报错）。值是 `{{#if ...}}` 的判断部分（不带 `{{ }}`），按 Handlebars
真值规则求值：`false`、缺失、空串、`0`、空数组都算假。典型用法是配合
`_variables.toml` 里的开关，只在需要时才生成某个文件：

```yaml
---
generateWhen: has_import          # has_import 为真才生成；为假则整个文件跳过
filename: "{{model_pascal}}ImportDTO.java"
---
```

```yaml
---
skipWhen: is_readonly             # generateWhen 的反向：为真则跳过
filename: "{{model_pascal}}Service.java"
---
```

被条件跳过的文件会在 `gen` 输出里单独标记 `[skipped: condition]`，与"已存在
而跳过"区分开。`validate` 会检查条件里引用的变量是否声明 —— 拼错的变量在生成时
会被当成假而**静默跳过**，所以务必先 `validate`。

### 内置上下文

模板里永远可用：

- `{{model}}`、`{{model_pascal}}`、`{{model_snake}}`、`{{model_camel}}`、
  `{{model_kebab}}`
- `{{table}}`、`{{table_comment}}`（表/实体业务说明，可选；`--table-comment`、JSON
  `table_comment` 或省略为空串）、`{{package}}`、`{{package_path}}`（点替换为斜杠）
- `{{fields}}` —— 用 `{{#each fields}}` 遍历；每一项暴露 `name`、
  `name_pascal`、`name_snake`、`name_camel`、`name_kebab`、`type`、
  `is_pk`、`required`、`comment`、`length`、`unique`、`default`。后四项
  来自 JSON `--file`（见下文 FieldSpec）；`--fields` DSL 不带这些元数据，
  此时 `comment` 为空串、`length`/`default` 为 `null`、`unique` 为 `false`。
  用 DDL 模板生成建表语句时正好用到 `comment`/`length`/`unique`。
- `{{pk_field}}`、`{{pk_field_type}}`、`{{pk_field_pascal}}` —— 由主表
  `fields` 中 `is_pk: true` 的字段推导（camelCase 名、原始类型、PascalCase
  名）；若无主键标记则默认为 `id` / `Long` / `Id`。
- `{{is_sub}}`、`{{sub_table}}`、`{{sub_table_comment}}`、`{{sub_fields}}` ——
  主子表：JSON 含 `sub` 块时为真并填充；否则 `is_sub` 为 false，其余为空。
  `sub_fields` 与 `fields` 同为对象数组。另含 `sub_model` 及 `sub_model_*` 大小写变体、
  `sub_model_fk`（子表外键 camelCase）、`sub_model_fk_pascal`（供 Java setter 使用）。
- `{{git_user_name}}`、`{{git_user_email}}`、`{{user_name}}`、`{{user_email}}`
- `{{date}}`、`{{datetime}}`、`{{year}}`

Helper：`pascal_case`、`snake_case`、`camel_case`、`kebab_case`（例如
`{{pascal_case "hello_world"}}` → `HelloWorld`）；`single_brace`、`double_brace`
（输出一层/两层大括号，用于 MyBatis 与 Vue 占位符）：

- `{{single_brace name_camel}}` → `{userId}`；模板里写 `#{{single_brace …}}` 得 `#{…}`，
  `${{single_brace …}}` 得 `${…}`（支持 `#{}` 与 `${}` 两种 MyBatis 写法）。
- `{{double_brace name_camel}}` → `{{userName}}`（Vue 模板插值）。

### 每次调用的变量（`_variables.toml`）

在 `.crud/templates/_variables.toml` 里声明这套模板需要哪些开关：

```toml
[has_import]
description = "是否生成导入按钮和 importExcel 接口"
type        = "bool"          # bool | string | number
default     = false

[has_export]
description = "是否生成导出接口"
type        = "bool"
default     = false
```

gen 时传值：

```bash
crud-cli gen User --fields "..." --package ... --table ... \
  --table-comment "系统用户" --var has_import=true
```

优先级：`--var` > JSON `variables` > schema `default`。缺失 required → 报错；
传了未声明的 key → 报错。

`description` 字段是给 Agent 读的契约 —— Agent 在生成命令前看一眼这个文件，
就知道有哪些开关、各自什么含义、类型是什么。

### JSON 实体输入

完整 schema 参考：[agent-resources/json-entity-input.md](agent-resources/json-entity-input.md)（面向 LLM Agent 的精简英文 spec，与 MCP server 对外提供的内容同源）。

需要更丰富的字段元数据时用 `--file`。每个字段（FieldSpec）支持 `name`、
`type`、`is_pk`、`required`、`length`、`unique`、`default`、`comment`，以及
自由形式的 `extra`；这些都会进入 `{{#each fields}}` 上下文。

```json
{
  "name": "User",
  "table": "sys_user",
  "table_comment": "系统用户",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "主键" },
    { "name": "email", "type": "String", "length": 128, "unique": true, "comment": "登录邮箱" }
  ],
  "variables": {
    "has_import": true
  }
}
```

主子表示例（`sub` 与顶层 `name`/`table`/`fields` 对称）：

```json
{
  "name": "Order",
  "table": "biz_order",
  "package": "com.acme.demo",
  "fields": [
    { "name": "order_id", "type": "Long", "is_pk": true, "comment": "订单主键" }
  ],
  "sub": {
    "name": "OrderItem",
    "table": "biz_order_item",
    "table_comment": "订单明细",
    "fk_field": "order_id",
    "fields": [
      { "name": "item_id", "type": "Long", "is_pk": true, "comment": "明细主键" },
      { "name": "order_id", "type": "Long", "comment": "订单外键" }
    ]
  }
}
```

```bash
crud-cli gen --file user.json
```

CLI flag（`--name`、`--package`、`--table`、`--table-comment`、`--var`）覆盖 JSON 里的同名值。

## 配置文件

| 文件 | 范围 | 入库 | 内容 |
|---|---|---|---|
| `.crud/setup.toml` | 项目 | 是 | `[project]`、`[paths.lang]`、`[paths.aux]`、`[variables]`、`[templates.outputs]`、`[type_map]` |
| `.crud/setup.user.toml` | 开发者 | 否 | `[user]`、`[overwrite]`、`[scope]` |
| `.crud/templates/_variables.toml` | 项目 | 是 | 每次调用变量的 schema |
| `.crud/templates/**/*.hbs` | 项目 | 是 | 模板 |
| `.crud/templates/.crudignore` | 项目 | 是 | 排除特定模板 |
| `~/.crud/config.toml` | 全局 | 否 | `[templates].repo` —— `template install` 的默认 GitHub 仓库 |
| `~/.crud/templates/<name>/<version>/` | 全局 | 否 | 已安装的模板包 |

所有 TOML schema 都是 `deny_unknown_fields` —— 拼错或漂写会立刻报错，
不会静默改变行为。

## MCP server

`crud-cli` 内置 MCP server，让 AI Agent 通过工具调用驱动代码生成，而不必走 shell。

```bash
cargo build --release --features full   # 或直接安装预编译二进制
crud-cli mcp
```

在 MCP 客户端中配置 `command: "crud-cli"`、`args: ["mcp", "--path", "/abs/path/to/project"]` 即可。服务端对外暴露工具（`crud_describe_templates`、`crud_preview`、`crud_generate` 等）、资源（`crud://schema/entity`、`crud://templates/variables` 等），以及 `crud_template_authoring` prompt，内容均来自 [`agent-resources/`](agent-resources/)。

完整说明：[docs/mcp-server.md](docs/mcp-server.md)。

## Agent 模式

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- 成功：exit 0，stdout 空。
- 失败：exit 非 0，stderr 输出单个 JSON 对象（`code`、`message`、`flag`、
  `value`、`remediation`，以及场景相关的 `details`），可直接回喂给模型。

## 架构

单 crate、三层模块，上层两层用 Cargo feature 开关，使用方按需付出依赖代价。

- `src/core/` —— 纯逻辑：配置解析、路径解析、模板引擎、事务写盘、validator、变量 schema、`thiserror` 类型化错误。不依赖 clap / inquire / tokio。
- `src/cli/` —— feature `cli`（默认）。`clap` 命令行、`inquire` 向导、Agent JSON 输出、人类可读输出。依赖 `core`。
- `src/mcp/` —— feature `mcp`。基于 `rmcp` + `tokio` 构建的 MCP server（`crud-cli mcp`），通过 stdio 把同一套 `core` API 提供给 LLM Agent；以 MCP prompts / resources 形式暴露 [`agent-resources/`](agent-resources/) 下的机器可读 spec。只依赖 `core`，永不依赖 `cli`。

Feature 组合：`default = ["cli"]`、`cli`、`mcp`、`full = ["cli", "mcp"]`。使用 `--no-default-features` 可把 `crud_cli` 作为纯库引入，不带 clap / inquire / tokio。

## 测试

```bash
cargo test            # 单元 + 集成 + 契约测试
```

契约测试（`tests/contracts/`）锁定 Agent 面向的接口：panic 行为、
JSON 错误 envelope 形状、setup 写盘的字节一致性。

## 许可证

MIT —— 见 [LICENSE](./LICENSE)。Copyright (c) 2026 Yujie Jin。
