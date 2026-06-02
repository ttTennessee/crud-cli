# JSON 实体输入

> **Other languages:** [English](../json-entity-input.md)

如何编写供 `crud-cli gen --file <path>.json` 使用的实体 JSON。

Schema 使用 `deny_unknown_fields`：每一层只能使用下文列出的属性。

---

## 快速开始

```bash
crud-cli gen --file entity.json
crud-cli gen --file entity.json --name User --package com.acme.app --table sys_user \
  --table-comment "系统用户" --var has_import=true
```

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

若模板包使用 `generateWhen` / `skipWhen`，请先执行 `crud-cli validate` 再 `gen`。

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

**不要**添加未列出的顶层键。**不要**在 JSON 里写 `is_sub`；需要主子表时提供 `sub` 即可。

---

## `sub` 对象（主子表）

| 属性 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 子实体名，如 `OrderItem` |
| `table` | 是 | 子表物理表名 |
| `fk_field` | 是 | 子表外键列名，如 `order_id` |
| `fields` | 是 | 字段对象数组（与主表 `fields` 结构相同） |
| `table_comment` | 否 | 子表业务说明 |

主子表仅支持 `--file`，不支持 `--fields` DSL。

---

## 字段对象（`fields` / `sub.fields`）

| 属性 | 必填 | 说明 |
|------|------|------|
| `name` | 是 | 列名：字母开头，仅含字母、数字、下划线 |
| `type` | 是 | 模板 `_field_types.toml` 中的类型，见下文 |
| `is_pk` | 否 | 是否主键（默认 `false`） |
| `required` | 否 | 是否必填（默认 `false`） |
| `comment` | 否 | 注释/文案 |
| `length` | 否 | 长度（建表等） |
| `unique` | 否 | 唯一约束 |
| `default` | 否 | 默认值（任意 JSON 值） |
| `extra` | 否 | 模板约定的扩展属性，见下文 |

**保留的 `name`：** 下列名称不能作为列名（否则校验失败）：  
`model`、`table`、`table_comment`、`package`、`package_path`、`fields` 等工具保留名。

主表建议恰有一个字段设置 `"is_pk": true`，以便正确识别主键。

---

## `variables` 对象

模板在所用模板包内的 `_variables.toml` 中声明需要哪些开关；JSON 里只能填写其中出现的键。

```json
"variables": {
  "module_name": "system",
  "function_name": "用户管理",
  "has_import": true
}
```

- 打开 `_variables.toml` 查看允许的键、类型（`bool` | `string` | `number`）和默认值。
- **优先级：** `--var` > JSON `variables` > schema 默认值。
- 未声明的键会报错。

---

## 字段 `type`

`type` 必须是当前模板目录下 `_field_types.toml` 中的 canonical 名或 alias：

| 项目配置 | 模板目录 |
|----------|----------|
| `.crud/setup.toml` 中 `[project] template = "name@version"` | `~/.crud/templates/<name>/<version>/` |
| 未固定 template | 项目内 `.crud/templates/` |

---

## 字段 `extra`

仅当所用模板文档约定了 `extra` 键时才需要填写（如 RuoYi）。常见示例：

| 键 | 类型 | 说明 |
|----|------|------|
| `query` | bool | 参与查询 |
| `query_like` | bool | LIKE 查询 |
| `query_between` | bool | 范围查询 |
| `list` | bool | 列表/导出列 |
| `insert` | bool | 新增/编辑表单 |
| `required` | bool | 必填 |
| `is_super` | bool | 基类已有字段 |
| `auto_increment` | bool | 自增主键 |
| `dict_type` | string | 字典类型 |
| `read_converter_exp` | string | Excel 转换 |
| `is_datetime`、`is_textarea` 等 | bool | 控件类型 |
| `ts_type` | string | TS 类型覆盖 |

```json
{
  "name": "status",
  "type": "int",
  "comment": "状态",
  "extra": { "query": true, "list": true, "dict_type": "sys_normal_disable" }
}
```

只使用模板文档中列出的 `extra` 键；未使用的键可能被忽略。

---

## 主子表示例

```json
{
  "name": "Order",
  "table": "biz_order",
  "package": "com.acme.demo",
  "table_comment": "订单",
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
  },
  "variables": {
    "module_name": "business",
    "function_name": "订单管理",
    "permission_prefix": "business:order"
  }
}
```

---

## 命令行覆盖

| 参数 | 覆盖项 |
|------|--------|
| `--name` | `name` |
| `--package` | `package` |
| `--table` | `table` |
| `--table-comment` | `table_comment` |
| `--var key=value` | `variables[key]` |

`fields`、`sub` 及字段明细**只能**写在 JSON 中。

---

## 常见错误

| 提示 | 排查 |
|------|------|
| `unknown field` | 属性名拼错，或该层级不允许此键 |
| `unsupported`（字段类型） | `type` 不在 `_field_types.toml` 中 |
| `undeclared variable` | `variables` / `--var` 的键未在 `_variables.toml` 声明 |
| `reserved_field_name` | 列名 `name` 与保留名冲突 |
| `variable shadows built-in` | `variables` 使用了工具保留名 — 仅使用 `_variables.toml` 中的键 |
| 文件被跳过且无报错 | 运行 `validate`；模板条件引用了未提供的变量 |

---

## 延伸阅读

- [README.zh.md](../../README.zh.md) — CLI 与模板说明（面向模板作者）
- [文档索引](../README.md)
