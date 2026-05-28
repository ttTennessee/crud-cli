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
- `crud-cli validate` — 上线前体检：Handlebars 语法、未声明变量、
  YAML front-matter、`filename`/`basePath` 安全性、fixture 渲染。
- Front-matter 三件套 `basePath` / `filename` / `overwrite`，且自动按
  `java/`、`resources/`、`doc/`、`vue/`、`react/`、`nest/` 前缀重定位到项目布局。
- `_variables.toml` schema：声明每次调用的开关变量（类型、默认值、必填、
  自然语言描述，最后这条是给 Agent 读的契约）。
- 两阶段事务式写盘 —— 任一文件冲突，整批回滚，磁盘上不留半成品。
- Agent 模式（`--agent`）：错误以结构化 JSON 输出到 stderr，成功时 stdout 为空。

尚未实现：`template install`、`template list`。

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

项目配置（checked in，团队共享）：

```bash
crud-cli setup --project --backend spring-boot --frontend vue \
  --component-library element-plus
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

输出落到 `<java_base>/com/acme/demo/controller/UserController.java`。

### 4. 提交前校验

```bash
crud-cli validate
# validate ok: 1 templates
```

## 路径系统

模板本身是项目无关的；具体落到哪里由 `.crud/setup.toml [paths]` 决定。
模板路径里出现的"约定前缀"会被替换成配置的真实目录：

| 模板前缀 | 配置项 | SpringBoot 默认 | Vue 默认 |
|---|---|---|---|
| `java/` | `paths.java_base` | `src/main/java` | — |
| `resources/` | `paths.resources_base` | `src/main/resources` | — |
| `doc/` | `paths.doc_base` | `doc/api` | — |
| `vue/` | `paths.vue_base` | — | `src/views` |
| `react/` | `paths.react_base` | — | `src/views` |
| `nest/` | `paths.nest_base` | `src`（Nest 后端） | — |

多模块/monorepo 项目，按布局覆写：

```toml
[paths]
java_base = "backend/api/src/main/java"
resources_base = "backend/api/src/main/resources"
doc_base = "docs/api"
```

`.crud/templates/java/Foo.hbs`（或在 front-matter 写
`basePath: "java/{{package_path}}/foo"`），不管宿主项目布局如何，
都会落到配置的 `java_base` 下。

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

### 内置上下文

模板里永远可用：

- `{{model}}`、`{{model_pascal}}`、`{{model_snake}}`、`{{model_camel}}`、
  `{{model_kebab}}`
- `{{table}}`、`{{package}}`、`{{package_path}}`（点替换为斜杠）
- `{{fields}}` —— 用 `{{#each fields}}` 遍历；每一项暴露 `name`、
  `name_pascal`、`name_snake`、`name_camel`、`name_kebab`、`type`、
  `is_pk`、`nullable`
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

[table_comment]
description = "表的业务说明，用于 Swagger 注解和类文档"
type        = "string"
required    = true
```

gen 时传值：

```bash
crud-cli gen User --fields "..." --package ... --table ... \
  --var has_import=true --var table_comment="系统用户"
```

优先级：`--var` > JSON `variables` > schema `default`。缺失 required → 报错；
传了未声明的 key → 报错。

`description` 字段是给 Agent 读的契约 —— Agent 在生成命令前看一眼这个文件，
就知道有哪些开关、各自什么含义、类型是什么。

### JSON 实体输入

需要更丰富的字段元数据时用 `--file`：

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true },
    { "name": "email", "type": "String", "extra": { "unique": true } }
  ],
  "variables": {
    "has_import": true,
    "table_comment": "系统用户"
  }
}
```

```bash
crud-cli gen --file user.json
```

CLI flag（`--name`、`--package`、`--table`、`--var`）覆盖 JSON 里的同名值。

## 配置文件

| 文件 | 范围 | 入库 | 内容 |
|---|---|---|---|
| `.crud/setup.toml` | 项目 | 是 | `[project]`、`[paths]`、`[variables]`、`[templates.outputs]` |
| `.crud/setup.user.toml` | 开发者 | 否 | `[user]`、`[overwrite]`、`[scope]` |
| `.crud/templates/_variables.toml` | 项目 | 是 | 每次调用变量的 schema |
| `.crud/templates/**/*.hbs` | 项目 | 是 | 模板 |
| `.crud/templates/.crudignore` | 项目 | 是 | 排除特定模板 |

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

MIT OR Apache-2.0
