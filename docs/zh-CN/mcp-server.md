# MCP Server（`crud-cli mcp`）

> **Other languages:** [English](../mcp-server.md)

通过 [Model Context Protocol](https://modelcontextprotocol.io/) 暴露 `crud-cli` 的代码生成能力，供 Cursor 等 Agent 调用。统一入口为 **`crud-cli mcp`**（stdio）。

## 构建与运行

```bash
cargo build --release --features full
crud-cli mcp
```

`full` = `cli` + `mcp`（与 `--features "cli,mcp"` 相同）。默认 `cargo build` 不含 MCP，需显式启用上述 feature。

MCP 客户端配置示例：

```json
{
  "mcpServers": {
    "crud-cli": {
      "command": "/path/to/crud-cli",
      "args": ["mcp"],
      "cwd": "/path/to/your/project"
    }
  }
}
```

`cwd` 必须指向已执行 `crud-cli setup` 且包含 `.crud/templates/` 或已 `template use` 的项目根目录。

## 推荐工作流（代码生成）

1. **读取资源**（或调用 `describe_templates`）  
   - `crud://templates/variables` — `_variables.toml`  
   - `crud://templates/field-types` — `_field_types.toml`  
   - `crud://schema/entity` — [entity.json 规范](json-entity-input.md)  
   - `crud://builtins` — 内置/保留变量名  

2. **编写 entity.json**（Agent 根据上述 schema 生成）

3. **`validate_entity`** — 校验 JSON（不落盘）

4. **`preview`** — 可选；`type=ddl` 仅预览建表 DDL（见下文 `ddl/` 前缀）

5. **`generate`** — 校验通过后写入项目

## MCP 工具

| 工具 | 说明 |
|------|------|
| `describe_templates` | 聚合返回 `variables` / `field_types`（由 TOML schema 解析后的 JSON）、类型前缀、`paths` 映射 |
| `validate_entity` | 校验 `entity_json` 字符串 |
| `preview` | 渲染预览（`type` 可选，如 `ddl`） |
| `generate` | 生成并落盘（`force` 可选） |

## MCP 资源（`crud://`）

| URI | 内容 |
|-----|------|
| `crud://templates/variables` | `_variables.toml` |
| `crud://templates/field-types` | `_field_types.toml` |
| `crud://schema/entity` | entity.json 文档 |
| `crud://builtins` | 保留名 JSON |
| `crud://docs/template-authoring` | 模板编写指南 |

## MCP Prompts

| 名称 | 说明 |
|------|------|
| `template_authoring` | 一次性模板包编写指南（对应 [template-authoring.md](template-authoring.md)） |

## DDL 与数据 SQL 分离

- **`ddl/`** — 建表 DDL（如 `schema.sql.hbs`），可用 `preview` / `gen` 的 `type=ddl` 单独预览。  
- **`sql/`** — 数据/菜单类 SQL（如 `menu.sql.hbs`）。  

两者在 `setup.toml` 的 `[paths.aux]` 中可映射到同一物理目录（默认 `ddl` → `sql` 输出目录）。

CLI 等价命令：

```bash
crud-cli gen --file entity.json --type ddl --stdout
```

## 架构说明

- MCP 层使用 [rmcp](https://github.com/modelcontextprotocol/rust-sdk)（stdio + tokio），由 `crud-cli mcp` 子命令启动。  
- 生成/校验逻辑在 `crud_cli::core`（同步）；MCP 工具通过 `spawn_blocking` 调用，不引入 async 进 core。  
- 默认构建（`default = ["cli"]`）不包含 MCP 依赖；需要 MCP 时使用 `--features full`。
