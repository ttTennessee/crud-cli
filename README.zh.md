# crud-cli

[![CI](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/ttTennessee/crud-cli/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](./LICENSE)

一个用 Rust 编写的命令行工具，配合 AI 编程 Agent（Claude Code、Cline、Copilot 等）
生成后台管理系统的 CRUD 脚手架，**避免** Agent 在模板样板代码上浪费 Token。

Agent 只下发一条短命令 + 结构化数据，`crud-cli` 在本地完成模板渲染。
单次 CRUD 的 Token 成本可从 2000+ 降到 ~50，约 **40 倍** 缩减，
同时保证产物与项目既有代码风格逐字节一致。

[English](./README.md)

## 当前状态

已实现：

- `crud-cli setup` — 交互式向导 + 非交互式 flag 模式。支持写入项目级
  `.crud/setup.toml` 或开发者级 `.crud/setup.user.toml`。
- `crud-cli gen` — 用字段 DSL 或 JSON 文件渲染模板。支持通过可重复的
  `--var key=value` 或 JSON 中的 `variables` 字段注入每次调用的变量。
  `--dry-run` 只列出将写入的文件不落盘；`--stdout` 把渲染结果直接打到标准输出
  而不写文件（配合 `--type sql` 可让 Agent 先把建表 SQL 给用户确认，再正式生成）。
- `crud-cli validate` — 上线前体检：Handlebars 语法、未声明变量、
  YAML front-matter、`filename`/`basePath` 安全性、fixture 渲染。
- Front-matter 三件套 `basePath` / `filename` / `overwrite`，且自动按
  `java/`、`resources/`、`doc/`、`vue/`、`react/`、`nest/` 前缀重定位到项目布局。
- `_variables.toml` schema：声明每次调用的开关变量（类型、默认值、必填、
  自然语言描述，最后这条是给 Agent 读的契约）。
- 两阶段事务式写盘 —— 任一文件冲突，整批回滚，磁盘上不留半成品。
- Agent 模式（`--agent`）：错误以结构化 JSON 输出到 stderr，成功时 stdout 为空。
- `crud-cli template install` —— 从 GitHub 仓库下载模板包到
  `~/.crud/templates/<name>/<version>/`。交互式选择名称/版本（版本会标注
  已安装 / 本地已改 / 仓库有更新 等状态），并可选择叠加共享的 `doc/`。
  脚本化用法：`template install name@version`。
- `crud-cli template list` —— 列出已安装的模板包。
- `crud-cli template use <name>[@version]` —— 把项目的 `[project].template`
  指向某个已安装模板包（同步 backend/frontend）。

## 安装

需要 Rust ≥ 1.75。

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release
# 二进制位于 ./target/release/crud-cli
```

`cargo install` 和预编译二进制会在 v0.1 发布时提供。

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
  `is_pk`、`nullable`、`comment`、`length`、`unique`、`default`。后四项
  来自 JSON `--file`（见下文 FieldSpec）；`--fields` DSL 不带这些元数据，
  此时 `comment` 为空串、`length`/`default` 为 `null`、`unique` 为 `false`。
  用 DDL 模板生成建表语句时正好用到 `comment`/`length`/`unique`。
- `{{git_user_name}}`、`{{git_user_email}}`、`{{user_name}}`、`{{user_email}}`
- `{{date}}`、`{{datetime}}`、`{{year}}`

Helper：`pascal_case`、`snake_case`、`camel_case`、`kebab_case`（例如
`{{pascal_case "hello_world"}}` → `HelloWorld`）。

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

需要更丰富的字段元数据时用 `--file`。每个字段（FieldSpec）支持 `name`、
`type`、`is_pk`、`nullable`、`length`、`unique`、`default`、`comment`，以及
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

## Agent 模式

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- 成功：exit 0，stdout 空。
- 失败：exit 非 0，stderr 输出单个 JSON 对象（`code`、`message`、`flag`、
  `value`、`remediation`，以及场景相关的 `details`），可直接回喂给模型。

## 架构

`core` 与 `cli` 严格分层，为未来的 MCP Server 复用预留接口：

- `src/core/` —— 纯逻辑：配置解析、路径解析、模板引擎、事务写盘、
  validator、变量 schema、`thiserror` 类型化错误。不依赖 clap / inquire。
- `src/cli/` —— `clap` 命令行、`inquire` 向导、Agent JSON 输出、
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

MIT —— 见 [LICENSE](./LICENSE)。Copyright (c) 2026 Yujie Jin。
