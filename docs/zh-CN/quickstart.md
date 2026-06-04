# 快速开始

**Languages:** [English](../quickstart.md) · 简体中文

本文档覆盖安装与日常 CLI 使用流程。AI Agent 驱动的 MCP server 见 [mcp.md](mcp.md)。

## 安装

### 预编译二进制（推荐）

到 [Releases](https://github.com/ttTennessee/crud-cli/releases) 下载对应平台的压缩包，解压后把 `crud-cli`（Windows 为 `crud-cli.exe`）放进 `PATH` 任一目录即可。

一键安装：

```bash
# Linux / macOS
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.sh | sh
```

```powershell
# Windows (PowerShell)
irm https://github.com/ttTennessee/crud-cli/releases/latest/download/crud-cli-installer.ps1 | iex
```

预编译二进制已包含 MCP server（`crud-cli mcp`）。

### 从源码构建

需要较新的 Rust stable 工具链（项目未承诺 MSRV）。

```bash
git clone https://github.com/ttTennessee/crud-cli.git
cd crud-cli
cargo build --release                 # 仅 CLI
cargo build --release --features full # CLI + MCP server
```

二进制位于 `./target/release/crud-cli`。

## 端到端走查

从零到生成代码，跑通一遍：

### 1. 初始化项目配置

`crud-cli setup` 会写入 `.crud/setup.toml`（共享，提交到版本库）和可选的 `.crud/setup.user.toml`（每位开发者本地，gitignored）。

非交互式：

```bash
crud-cli setup --project --backend java --frontend vue \
  --lang java=src/main/java --lang vue=src/views \
  --aux resources=src/main/resources --aux doc=doc/api
```

交互式（去掉 `--project` 与各 flag 即进入向导）：

```bash
crud-cli setup
```

每位开发者的本地覆盖（用户名会出现在生成文件头里；overwrite 策略控制 `gen` 何时允许覆盖已有文件）：

```bash
crud-cli setup --user-name "Alice" --user-email alice@example.com \
  --overwrite-policy force-only --enabled-types backend
```

### 2. 添加模板

把 `.hbs` 文件放进 `.crud/templates/`。**第一段路径**（这里是 `java/`）是一个**前缀**，会按 `setup.toml` 的 `[paths.lang]` 解析。

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

模板编写完整参考（front-matter、helpers、`_variables.toml` 等）目前仍在 [主 README](../../README.zh.md) 里；后续会迁到 [templates.md](templates.md)。

### 3. 校验

生成前以及提交模板改动前都建议先跑：

```bash
crud-cli validate
# validate ok: 1 templates
```

会检出：Handlebars 语法错误、引用了未声明的变量、`filename` / `basePath` 不安全、YAML front-matter 损坏。

### 4. 生成

字段 DSL —— 简洁，适合快速脚手架：

```bash
crud-cli gen User --table sys_user --package com.acme.demo \
  --fields "name:String,age:Integer"
```

产物位于 `<paths.lang.java>/com/acme/demo/controller/UserController.java`。

JSON 实体输入 —— 需要更丰富的字段元数据（注释、长度、唯一、默认值、主子表、字段级 extras）时使用：

```bash
crud-cli gen --file user.json
```

`entity.json` 规范见 [entity.md](entity.md)。

## CLI 子命令参考

下面每个子命令只给典型用途。完整 flag 列表请跑 `crud-cli <subcommand> --help` —— 这是和二进制同源、不会漂移的权威清单。

### `crud-cli setup`

交互式向导或非交互式 flag 模式，写入 `.crud/setup.toml`（共享）和/或 `.crud/setup.user.toml`（每位开发者）。每个项目跑一次，需要用户级配置的开发者再跑一次。

### `crud-cli gen <Name>`

把模板渲染到项目目录。

- **字段 DSL**（`--fields "name:type,..."`）：快速脚手架。
- **JSON 文件**（`--file entity.json`）：丰富字段元数据或主子表。
- **每次调用的变量**：通过可重复的 `--var key=value` 或 JSON 中的 `variables` 对象传入 —— 取值来自 `_variables.toml` 的声明。
- **`--dry-run`**：列出将写入的文件但不落盘。
- **`--stdout`**：把渲染结果打到标准输出而不写文件。配合 `--type sql`，可以让 Agent 在正式生成前先把 DDL 给用户确认。
- **事务式**：任一文件冲突时整批回滚 —— 磁盘上不会留下半成品。

### `crud-cli validate`

对 `.crud/templates/` 做上线前体检：Handlebars 语法、未声明变量引用、YAML front-matter、`filename` / `basePath` 安全性、fixture 渲染。建议在提交前和 CI 里都跑。

### `crud-cli template install [<name>[@<version>]]`

从 GitHub 仓库下载模板包到 `~/.crud/templates/<name>/<version>/`。交互式选择名称/版本（版本列表会标注 已安装 / 本地已改 / 仓库有更新 等状态）。默认仓库可在 `~/.crud/config.toml` 的 `[templates].repo` 配置。

### `crud-cli template list`

列出 `~/.crud/templates/` 下已安装的模板包。

### `crud-cli template use <name>[@<version>]`

把当前项目的 `[project].template` 指向某个已安装的模板包（同步 backend / frontend 选择）。

### `crud-cli mcp`

通过 stdio 启动内置 MCP server 供 AI Agent 调用。需要构建时启用 `--features full`（预编译二进制已满足）。配置和工具参考见 [mcp.md](mcp.md)。

## Agent 模式（`--agent`）

一个全局 flag，会把 CLI 输出切换成机器可读形式 —— 适用于 Agent 直接 shell 调 `crud-cli`（不走 MCP server）的场景：

```bash
crud-cli --agent gen User --fields "id:Long" --package com.x --table u
```

- **成功**：exit 0，stdout 为空。
- **失败**：exit 非 0，stderr 输出单个 JSON 对象，包含 `code`、`message`、`flag`、`value`、`remediation`，以及场景相关的 `details`。可直接回喂给模型。

绝大多数 Agent 集成场景下，MCP server 是更好的选择 —— 结构化工具调用、不用解析 CLI 输出。`--agent` 模式留给脚本和 CI。
