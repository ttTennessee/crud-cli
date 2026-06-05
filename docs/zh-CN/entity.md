# crud-cli JSON 实体输入规范

**Languages:** [English](../entity.md) · 简体中文

`crud-cli gen --file <path>` 和 MCP 的 `crud_preview` / `crud_generate` 工具接收的 `entity.json` 的 schema。

所有对象都使用 `deny_unknown_fields` —— 凡是下表未列出的键都会被拒绝。

## 顶层

| Key | Type | Required | 说明 |
|---|---|---|---|
| `name` | string | 是 | 实体 / 类名（例如 `User`） |
| `table` | string | 是 | 主表物理名 |
| `package` | string | 是 | 服务端包名（例如 Java 的 `com.acme.demo`） |
| `fields` | FieldSpec[] | 是 | 主表字段 |
| `table_comment` | string | 否 | 主表描述 |
| `sub` | SubSpec | 否 | 主子表块；只要存在就隐含 `is_sub = true`（**不要**自己设 `is_sub`） |
| `variables` | object | 否 | 当前模板的 `_variables.toml` 声明键的取值 |

## `SubSpec`

| Key | Type | Required | 说明 |
|---|---|---|---|
| `name` | string | 是 | 子实体名（例如 `OrderItem`） |
| `table` | string | 是 | 子表名 |
| `fk_field` | string | 是 | 子表上的外键列（例如 `order_id`） |
| `fields` | FieldSpec[] | 是 | 子表字段 |
| `table_comment` | string | 否 | 子表描述 |

## `FieldSpec`

`fields` 和 `sub.fields` 都使用此结构。

| Key | Type | Required | 说明 |
|---|---|---|---|
| `name` | string | 是 | 列名。必须字母开头，只允许 `[A-Za-z0-9_]`。不能是以下保留标识符之一：`model`、`table`、`table_comment`、`package`、`package_path`、`fields` |
| `type` | string | 是 | 当前模板 `_field_types.toml` 中的规范名或别名（也由 MCP `crud_describe_templates` 工具返回） |
| `is_pk` | bool | 否 | 默认 `false`。主表恰好把一个字段标为主键 |
| `required` | bool | 否 | 默认 `false` |
| `comment` | string | 否 | 标签 / 列注释 |
| `length` | number | 否 | DDL 长度 |
| `unique` | bool | 否 | 唯一约束 |
| `default` | any | 否 | 默认值（任意 JSON 类型） |
| `extra` | object | 否 | 模板特定的字段级 flag（见下） |

## `variables` 对象

当前模板 `_variables.toml` 声明的顶层开关（也由 MCP `crud_describe_templates` 工具返回）。未声明的键会被拒绝。允许的值类型：`bool` | `string` | `number`。

```json
"variables": { "module_name": "system", "has_import": true }
```

## `extra` 对象（字段级）

模板特定的字段级 flag。合法键由当前模板的 `_field_extra.toml` 声明；调用 MCP `crud_describe_templates` 工具并读取其 `field_extra` 字段，可以了解有哪些键、值类型，以及哪些字段类型要求必填。

```json
{ "name": "status", "type": "int", "extra": { "query": true, "dict_type": "sys_normal_disable" } }
```

`_field_extra.toml` 存在时，`crud_preview` 会对未知或缺少必填 extra 键的情况在返回结果的 `warnings` 数组中给出提示（非阻断——仍会继续生成）。

## 示例

### 平表

```json
{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long", "is_pk": true, "comment": "主键" },
    { "name": "email", "type": "String", "length": 128, "unique": true, "comment": "邮箱" }
  ]
}
```

### 主子表

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

## 错误

| 消息 | 原因 |
|---|---|
| `unknown field` | 该层级不允许此键（拼写错误或多余键） |
| `unsupported` | `type` 不在 `_field_types.toml` 中 |
| `undeclared variable` | `variables` 中的键不在 `_variables.toml` 里 |
| `reserved_field_name` | `FieldSpec.name` 与保留标识符冲突 |
| `variable shadows built-in` | `variables` 中的键使用了内置名 |
