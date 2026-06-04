# MCP server

**Languages:** [English](../mcp.md) · 简体中文

`crud-cli` 内置了 [Model Context Protocol](https://modelcontextprotocol.io/) server，AI Agent 可以通过结构化工具调用驱动代码生成，而不必走 shell。MCP server 复用与 CLI 完全相同的 `core` API —— 同一套模板、同一套校验、同一套写盘语义 —— 通过 stdio 对外暴露。

## 为什么用它

相比让 Agent 跑 `crud-cli gen ...`：

- **输入输出都是结构化的**。工具的入参是 JSON、返回值也是 JSON，不需要解析人类可读的 CLI 输出。
- **更便宜的往返成本**。`crud_describe_templates` 一次返回当前模板包的变量 schema 和字段类型 schema，Agent 无需通过读文件来发现这些信息。
- **写前先看**。`crud_preview` 校验 `entity.json` 并返回归一化后的字段表 —— 用户可以在任何文件写入磁盘**之前**确认字段类型和必填标志。
- **同样的写盘保证**。`crud_generate` 复用 CLI 那套两阶段事务式写盘；冲突时磁盘上不留半成品。

## 构建与启动

MCP server 由 `mcp` Cargo feature 开关控制。建议使用 [Releases](https://github.com/ttTennessee/crud-cli/releases) 的预编译二进制（构建时已带 `--features full`），或在本地构建：

```bash
cargo build --release --features full
./target/release/crud-cli mcp                # 在当前目录启动 stdio server
./target/release/crud-cli mcp --path /abs/path/to/project
```

server 通过 **stdio** 通信 —— 它不是常驻的网络守护进程。由 MCP 客户端（Claude Desktop、Cursor、Cline……）按需拉起。

### 项目根目录解析顺序

多个工具需要知道用哪个项目的 `.crud/setup.toml`。解析顺序：

1. `--path <DIR>` 参数（会被 canonicalize；必须存在且是目录）
2. 客户端通过 MCP `roots/list` 上报的 roots（如果客户端实现了 roots）
3. 进程 `cwd`，向上找 `.crud/setup.toml`；起点若在 `$HOME` 之下，则以 `$HOME` 为天花板停止上溯

`--path` 最显式，建议所有客户端配置里都写上。

## 客户端配置

### Claude Desktop / Claude Code（`claude_desktop_config.json`）

```json
{
  "mcpServers": {
    "crud-cli": {
      "command": "crud-cli",
      "args": ["mcp", "--path", "/abs/path/to/your/project"]
    }
  }
}
```

### Cursor（工作区下 `.cursor/mcp.json`）

```json
{
  "mcpServers": {
    "crud-cli": {
      "command": "crud-cli",
      "args": ["mcp", "--path", "${workspaceFolder}"]
    }
  }
}
```

### Cline / Continue / 其它通用 MCP 客户端

写法一致 —— `command` 指向 `PATH` 上的 `crud-cli` 二进制，`args` 传 `["mcp", "--path", "<abs-project-dir>"]`。客户端若支持 MCP `roots/list` 也可以省略 `--path`，但显式更稳妥。

## 工具

| 名称 | 用途 |
|---|---|
| `crud_describe_templates` | 返回当前模板包的 `_variables.toml` schema、`_field_types.toml` 别名、项目路径（`paths.lang` / `paths.aux`）以及解析后的项目元数据。编写 `entity.json` 时**先调它**。 |
| `crud_preview` | 校验 `entity.json` 并返回字段级归一化结构表供用户确认。不渲染、不落盘。 |
| `crud_generate` | 校验并把生成的文件写到项目目录。支持 `type` 过滤器（如 `ddl`）和 `force` 来绕过 overwrite 策略。使用与 `crud-cli gen` 同一套事务式写盘。 |

各工具的入参 / 返回 schema 通过 MCP `tools/list` 暴露给客户端 —— 实时 JSON schema 看客户端的工具检查器。

## 资源

| URI | MIME | 内容 |
|---|---|---|
| `crud://schema/entity` | `text/markdown` | `entity.json` 规范，单 markdown 文档。源同 [`agent-resources/entity.md`](../../agent-resources/entity.md)。 |
| `crud://schema/builtins` | `application/json` | 模板自动注入的保留变量名与字段标识符。适合需要在调 `crud_preview` 之前本地校验 `entity.json` 的客户端。 |

## Prompts

| 名称 | 用途 |
|---|---|
| `crud_template_authoring` | 一次性 prompt，把完整的模板编写指南作为 user message 返回。源同 [`agent-resources/template-authoring.md`](../../agent-resources/template-authoring.md)。Agent 准备写或改 `.hbs` 文件时用。 |

## 推荐 Agent 工作流

针对**生成代码到现有项目**：

1. **`crud_describe_templates`** → 拿到当前模板包的变量、字段类型、以及项目的路径布局。
2. **（可选）读 `crud://schema/entity`**，如果 Agent 对 `entity.json` 的结构不熟。
3. **撰写 `entity.json`**，依据用户意图 + 第 1 步拿到的 schema。
4. **`crud_preview`** → 给用户展示归一化后的字段表用于确认。需要时迭代。
5. **`crud_generate`** → 写文件。

针对**编写或改造模板包**：

1. **`crud_template_authoring` prompt** → 把编写指南加载进对话上下文。
2. 阅读目标项目里已有的手写文件（controller、service、前端页面），抓住代码风格。
3. 在 `.crud/templates/` 下编辑 `.hbs` 文件。
4. 本地跑 `crud-cli validate` 后再生成。

## 错误处理

工具调用因校验失败而返回时，错误体放在工具结果里（不走 MCP 协议错误）。结构与 CLI `--agent` 模式的错误信封一致：

```json
{
  "code": "...",
  "message": "...",
  "flag": "...",
  "value": "...",
  "remediation": "...",
  "details": { /* 场景相关 */ }
}
```

Agent 可以原样回喂给模型；`remediation` 字段刻意写得可执行。

硬故障（找不到项目根、读不了 `setup.toml` 等）则走 MCP 协议错误返回，带人类可读消息。

## 参见

- [`agent-resources/template-authoring.md`](../../agent-resources/template-authoring.md) —— `crud_template_authoring` prompt 的源文档
- [`agent-resources/entity.md`](../../agent-resources/entity.md) —— `crud://schema/entity` 的源文档
- [主 README](../../README.zh.md) —— 项目概览、安装、基本 CLI 用法
