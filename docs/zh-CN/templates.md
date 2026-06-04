# 模板编写规范

**Languages:** [English](../templates.md) · 简体中文

`crud-cli` 的 Handlebars（`.hbs`）模板包编写权威参考。

## 模板包结构

```
<bundle-root>/                   # 项目内：.crud/templates/  |  全局：~/.crud/templates/<name>/<version>/
  **/*.hbs                       # 模板；可选 YAML front-matter
  _variables.toml                # 每次调用的扩展顶层变量 schema
  _field_types.toml              # 允许的字段类型名
  type_map.toml                  # 可选；供 ty_map helper 使用
  .crudignore                    # 把文件排除在发现之外
```

模板第一段路径（例如 `java/`、`vue/`、`resources/`）是一个**前缀**，会按 `.crud/setup.toml` 的 `[paths.lang]` → `[paths.aux]` 顺序解析，并 rebase 到配置的目录下。

## Front-matter

任何 `.hbs` 文件顶部都可以放一段可选的 YAML：

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only
generateWhen: has_import
---
```

| 键 | 类型 | 说明 |
|---|---|---|
| `basePath` / `base_path` | string | 输出目录，相对于项目根。可引用内置或扩展的顶层变量。 |
| `filename` | string | 输出文件名。**只能是单段路径** —— 不允许 `/`、不允许 `..`。可引用变量。 |
| `overwrite` | enum | `never` \| `force-only` \| `always`。覆盖全局/用户默认值。 |
| `generateWhen` / `generate_when` | string | 仅在条件为真时生成。值是 `{{#if ...}}` 的内部表达式 —— 不要带外层 `{{ }}`。 |
| `skipWhen` / `skip_when` | string | 条件为真时**跳过**。与 `generateWhen` 互斥。 |

**真值判定**：`false`、缺失、`""`、`0`、`[]` 都为假。

**条件陷阱**：`generateWhen` / `skipWhen` 中未声明的变量会判定为假，导致文件**被静默跳过**。务必先跑 `crud-cli validate` 把拼写错误曝出来。

front-matter 里包含 `{{` 的值要加引号，避免 YAML 解析失败。

## 内置顶层变量

每次 `gen` 自动注入。**绝对不要**在 `_variables.toml` 或 `setup.toml` `[variables]` 中声明同名键 —— 会触发 `variable shadows built-in`。

### 实体 / 表

| 变量 | 类型 | 示例 |
|---|---|---|
| `model` | string | `User` |
| `model_pascal` | string | `User` |
| `model_snake` | string | `user` |
| `model_camel` | string | `user` |
| `model_kebab` | string | `user` |
| `table` | string | `sys_user` |
| `table_comment` | string | 未填则 `""` |
| `package` | string | `com.acme.demo` |
| `package_path` | string | `com/acme/demo` |

### 主键（从 `fields` 中 `is_pk: true` 的字段派生）

| 变量 | 类型 | 无主键时的默认值 |
|---|---|---|
| `pk_field` | string（camelCase） | `id` |
| `pk_field_type` | string | `Long` |
| `pk_field_pascal` | string | `Id` |

### 主子表

| 变量 | 类型 | 无主子表时 |
|---|---|---|
| `is_sub` | bool | `false` |
| `sub_table`、`sub_table_comment` | string | `""` |
| `sub_model`、`sub_model_snake`、`sub_model_pascal`、`sub_model_camel`、`sub_model_kebab` | string | `""` |
| `sub_model_fk` | string（camelCase 外键列） | `""` |
| `sub_model_fk_pascal` | string | `""` |
| `sub_fields` | array | `[]` |

### 字段数组

`fields` 和 `sub_fields` —— 用 `{{#each}}` 遍历；每项的字段结构见下文。

### 作者 / 时间

| 变量 | 来源 |
|---|---|
| `git_user_name`、`git_user_email` | git config |
| `user_name`、`user_email` | `.crud/setup.user.toml`；为空时回落到 git |
| `date` | `YYYY-MM-DD`（本地时区） |
| `datetime` | `YYYY-MM-DD HH:MM:SS`（本地时区） |
| `year` | 四位年份 |

### Handlebars `{{#each}}` 特殊变量（validator 已认可）

`this`、`@index`、`@key`、`@first`、`@last`、`@root`。

## 字段对象结构（`{{#each fields}}` / `{{#each sub_fields}}`）

每个字段必有的**默认属性**：

| 属性 | 类型 | 输入未提供时 |
|---|---|---|
| `name` | string | — |
| `name_pascal`、`name_snake`、`name_camel`、`name_kebab` | string | — |
| `type` | string | — |
| `is_pk` | bool | `false` |
| `required` | bool | `false` |
| `comment` | string | `""` |
| `length` | number \| null | `null` |
| `unique` | bool | `false` |
| `default` | any \| null | `null` |

`--fields` DSL 只会填充 `name`、`type`、`is_pk`。需要完整元数据时使用 JSON 输入（见 `entity` 资源）。

**扩展字段属性**：模板包的调用方可以传额外的 key/value，会被**展平**到每个字段对象上 —— 在 `{{#each fields}}` 中与默认属性同级访问。`validate` 静态检查不识别扩展键；如果某个模板包定义的扩展键触发 `unknown variable`，那就是和模板包契约的拼写不一致。

## 扩展顶层变量

供 front-matter、`{{#if}}` 和模板正文使用的自定义变量：

| 来源 | 用途 |
|---|---|
| `_variables.toml`（模板包根目录） | Schema：`type`、`default`、`required`、`description`。`description` 是调用方读取的契约。 |
| `.crud/setup.toml` `[variables]` | 项目级默认值 |
| `--var key=value` | 每次调用时覆盖 |
| JSON gen 输入中的 `variables` 对象 | 每次调用时覆盖 |

**优先级**：`--var` > JSON `variables` > schema `default`。

自定义名称**不得**与内置变量或 `fields` / `sub_fields` 冲突。`validate` 接受：内置变量 ∪ `_variables.toml` 声明 ∪ `[variables]` 键。

```toml
# _variables.toml
[has_import]
description = "Generate import button and importExcel endpoint"
type        = "bool"
default     = false

[module_name]
description = "Business module id for permission prefix and routes"
type        = "string"
required    = true
```

`_variables.toml` 的类型：`bool` \| `string` \| `number`。

## Helpers

### 大小写转换（单 string 参数）

| Helper | `hello_world` → |
|---|---|
| `pascal_case` | `HelloWorld` |
| `snake_case` | `hello_world` |
| `camel_case` | `helloWorld` |
| `kebab_case` | `hello-world` |

### 花括号包裹（不做 HTML 转义）

输出**不会被** HTML 转义 —— `<List<T>>` 原样穿透。下面这俩 helper 用来在生成产物中嵌入字面花括号，避免与 Handlebars 语法冲突：

| Helper | 当 `name_camel = userId` | 输出 |
|---|---|---|
| `single_brace` | `{{single_brace name_camel}}` | `{userId}` |
| `double_brace` | `{{double_brace name_camel}}` | `{{userId}}` |

MyBatis（`#` / `$` 写在 helper **外面**）：

```handlebars
WHERE id = #{{single_brace pk_field}}
ORDER BY ${{single_brace pk_field}}
```

Vue 字面插值：

```handlebars
<span>{{double_brace name_camel}}</span>
```

### `ty_map`

按当前模板包的 `type_map.toml` 把中性类型名映射为目标技术栈类型：

```handlebars
private {{ty_map type}} {{name_camel}};
```

未命中时，由 `.crud/setup.toml` 的 `[type_map].fallback` 决定行为：

| `fallback` | 行为 |
|---|---|
| `passthrough`（默认） | 原样输出该类型字符串 |
| `error` | 中止渲染 |
| 任意其它字符串 | 用该字面量替换 |

### 标准 Handlebars（validator 接受）

块：`{{#if}}` / `{{#unless}}` / `{{#each}}` / `{{#with}}`。
子表达式：`(eq a b)`、`(ne a b)`、`(and a b)`、`(or a b)`、`(not x)`。
路径：`../` 进入父上下文；`lookup` 做动态属性访问。

## 工作流

1. 读 `.crud/setup.toml`、`_variables.toml`、`_field_types.toml`，再读目标项目里**一两个手写**的同类文件 —— 这是代码风格的"标准答案"。
2. 写 `.hbs` 文件；在 `_variables.toml` 中声明扩展变量；通过 front-matter `basePath` / `filename` 安排输出路径。
3. `crud-cli validate` —— 捕获未知变量、缺失 helper、不安全文件名。
4. `crud-cli gen ... --dry-run`，再用 `--stdout` 对照手写样本检查。
5. 通过**改模板**来迭代，**不要**事后手改生成的代码。

## 错误目录

| 消息 | 原因 |
|---|---|
| `unknown variable` / `UnknownVariable` | 变量不在 内置 ∪ `_variables.toml` ∪ `[variables]` 中（或它是模板包定义的扩展字段键 —— 对照模板包文档检查拼写） |
| `variable shadows built-in` | `_variables.toml` 或 `[variables]` 用了保留的内置名 |
| `missing_helper` | helper 未注册，且不是 Handlebars 内置 |
| `helper not found`（渲染阶段） | 同上；或 `ty_map` 在 `fallback=error` 时遇到未映射的类型 |
| `[skipped: condition]` | `generateWhen` / `skipWhen` 判定为假 |
| `invalid filename` | front-matter `filename` 含 `/` 或 `..` |
| front-matter YAML 解析错误 | 值中含 `{{` 时要加引号 |
