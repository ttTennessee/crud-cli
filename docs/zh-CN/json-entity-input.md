# JSON 实体输入

> **Other languages:** [English](../json-entity-input.md)

供 MCP `preview` / `generate` 工具使用的 `entity.json`（CLI `--file` 同样接受）。

Schema 使用 `deny_unknown_fields`：每一层只能使用下文列出的属性。

最小示例：

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "主键" },
    { "name": "email", "type": "String", "comment": "邮箱" }
  ]
}
```

---

## 顶层对象

| 属性 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 实体/类名，如 `User`、`Order` |
| `table` | 是 | 主表物理表名 |
| `package` | 是 | 服务端包名（如 Java package） |
| `fields` | 是 | 字段对象数组，见下文 |
| `table_comment` | 否 | 主表业务说明 |
| `sub` | 否 | 主子表对象，见下文 |
| `variables` | 否 | 模板 `_variables.toml` 中已声明的变量取值 |

**不要**添加未列出的顶层键。**不要**写 `is_sub`；需要主子表时提供 `sub`（`--fields` 无法表达 `sub`）。

## `sub` 对象（主子表）

结构与顶层一致，额外多一个外键：

| 属性 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 子实体名，如 `OrderItem` |
| `table` | 是 | 子表物理表名 |
| `fk_field` | 是 | 子表外键列名，如 `order_id` |
| `fields` | 是 | 字段对象数组（与主表 `fields` 结构相同） |
| `table_comment` | 否 | 子表业务说明 |

## 字段对象（`fields` / `sub.fields`）

| 属性 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 列名：字母开头，仅含字母、数字、下划线；不能是保留名（`model`、`table`、`table_comment`、`package`、`package_path`、`fields`） |
| `type` | 是 | 当前模板 `_field_types.toml`（资源 `crud://templates/field-types`）中的 canonical 名或 alias |
| `is_pk` | 否 | 是否主键（默认 `false`；主表恰设一个） |
| `required` | 否 | 是否必填（默认 `false`） |
| `comment` | 否 | 注释/文案 |
| `length` | 否 | 长度（建表等） |
| `unique` | 否 | 唯一约束 |
| `default` | 否 | 默认值（任意 JSON 值） |
| `extra` | 否 | 模板约定的扩展属性，见下文 |

## `variables` 对象

模板在 `_variables.toml`（资源 `crud://templates/variables`）中声明的顶层开关；只能填写其中出现的键，未声明的键会报错。允许的类型（`bool` | `string` | `number`）与默认值见该文件。

```json
"variables": { "module_name": "system", "has_import": true }
```

## `extra` 对象

字段级扩展标志，仅当模板文档约定时才有意义（如 RuoYi：`query`、`list`、`insert`、`dict_type`、`ts_type` 等）。模板未使用的键会被透传，可能被忽略。

```json
{ "name": "status", "type": "int", "extra": { "query": true, "dict_type": "sys_normal_disable" } }
```

## 主子表示例

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
    "fk_field": "order_id",
    "fields": [
      { "name": "item_id", "type": "Long", "is_pk": true, "comment": "明细主键" },
      { "name": "order_id", "type": "Long", "comment": "订单外键" }
    ]
  },
  "variables": { "module_name": "business", "permission_prefix": "business:order" }
}
```

---

## 常见错误

| 提示 | 排查 |
|------|------|
| `unknown field` | 属性名拼错，或该层级不允许此键 |
| `unsupported`（字段类型） | `type` 不在 `_field_types.toml` 中 |
| `undeclared variable` | `variables` 的键未在 `_variables.toml` 声明 |
| `reserved_field_name` | 列名与保留名冲突 |
| `variable shadows built-in` | `variables` 使用了工具保留名 |

## 延伸阅读

- [MCP Server](mcp-server.md) — `preview` / `generate` 工具与 `crud://` 资源
- [文档索引](README.md)
