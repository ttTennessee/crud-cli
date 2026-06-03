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
      "args": ["mcp", "--path", "/path/to/your/project"]
    }
  }
}
```

Cursor 可使用 `"args": ["mcp", "--path", "${workspaceFolder}"]`。

项目根解析优先级：`--path` → MCP `roots/list`（客户端支持时）→ 进程 `cwd`（最后兜底）。从起始路径**向上**查找 `.crud/setup.toml`（在用户主目录内不会越过主目录；其他路径则止于当前盘符/卷根）。

仍可将 `cwd` 设为项目根作为兜底，但推荐 `--path` 或 `roots`，避免宿主把子进程起在非工作区目录。

## 推荐工作流（代码生成）

1. **调用 `crud_describe_templates`** 获取 `variables` / `field_types` schema、`paths`、`project` 等；再按需读取静态资源:
   - `crud://schema/entity` — [entity.json 规范](json-entity-input.md)  
   - `crud://schema/builtins` — 内置/保留变量名  

2. **编写 entity.json**（Agent 根据上述 schema 生成）

3. **`crud_preview`** — 校验 entity.json，并返回归一化后的字段结构表（不渲染代码、不落盘），供用户确认字段类型 / 必填 / 长度等

4. **`crud_generate`** — 确认无误后写入项目

> `crud_preview` 已合并原 `validate_entity` 的校验职责：解析、字段类型、变量出错时返回 `ok:false` 错误信息；校验通过则返回结构表。

## MCP 工具

| 工具 | 说明 |
|------|------|
| `crud_describe_templates` | 聚合返回 `variables` / `field_types`（由 TOML schema 解析后的 JSON）、类型前缀、`paths` 映射 |
| `crud_preview` | 校验 `entity_json` 并返回归一化后的字段结构：`fields`（机器可读）、`table_markdown`（给用户渲染确认）、`prompt`（展示指引）；不渲染代码、不落盘 |
| `crud_generate` | 生成并落盘（`type` / `force` 可选） |

### `crud_preview` 返回结构

```json
{
  "ok": true,
  "entity": { "name": "Order", "table": "biz_order", "pk": "orderId", "...": "..." },
  "fields": [
    { "name": "orderNo", "column": "order_no", "type": "String", "pk": false,
      "required": true, "length": 32, "default": null, "unique": false,
      "comment": "订单编号", "tags": ["insert", "list", "query"] }
  ],
  "sub": { "fk_field": "orderId", "fields": [/* 同上结构 */], "...": "..." },
  "table_markdown": "## Order (biz_order)\n\n### 主表字段\n\n| 字段名 | ... |\n...",
  "prompt": "Render `table_markdown` to the user ..."
}
```

- `fields` / `sub.fields` 为**归一化后**的值（驼峰 → 列名、字段类型经 `_field_types.toml` 归一、长度/默认填充），`name` 为稳定锚点，便于改回原 entity.json。
- `table_markdown` 由 `fields` 派生，供 Agent 直接渲染给用户确认。

## MCP 资源（`crud://`）

| URI | 内容 | MIME |
|-----|------|------|
| `crud://schema/entity` | entity.json 文档 | `text/markdown` |
| `crud://schema/builtins` | 保留名 | `application/json` |

> `variables` / `field_types` schema 不再作为资源暴露,改由 `crud_describe_templates` 工具统一返回（避免重复）。

## MCP Prompts

| 名称 | 说明 |
|------|------|
| `crud_template_authoring` | 一次性模板包编写指南（对应 [template-authoring.md](template-authoring.md)） |

## DDL 与数据 SQL 分离

- **`ddl/`** — 建表 DDL（如 `schema.sql.hbs`），可用 `crud_generate` 的 `type=ddl` 或 CLI `gen --type ddl` 单独生成。  
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
