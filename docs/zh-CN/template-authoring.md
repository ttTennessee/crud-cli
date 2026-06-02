# 模板编写

> **Other languages:** [English](../template-authoring.md)

面向模板作者与 AI Agent：如何编写、适配 `crud-cli` 使用的 Handlebars（`.hbs`）模板。

本文只说明模板语法、渲染上下文、内置 helper，以及 Agent 在动手写模板前应如何从目标项目收集信息。

---

## 快速开始

模板放在项目 `.crud/templates/` 或已安装的全局模板包 `~/.crud/templates/<name>/<version>/` 下。

```handlebars
---
basePath: "java/{{package_path}}/controller"
filename: "{{model_pascal}}Controller.java"
overwrite: force-only
---
package {{package}}.controller;

@RestController
@RequestMapping("/{{model_kebab}}")
public class {{model_pascal}}Controller {
    // ...
}
```

```bash
crud-cli validate          # 上线前体检
crud-cli gen User --fields "id:Long,name:String" --package com.acme.demo --table sys_user
```

---

## 模板包结构

一套模板包通常包含：

| 文件 / 目录 | 作用 |
|-------------|------|
| `**/*.hbs` | Handlebars 模板；可选 YAML front-matter 控制输出路径与条件生成 |
| `_variables.toml` | 声明**每次调用**的扩展顶层变量；供 Agent 与校验器读取 |
| `_field_types.toml` | 声明允许的字段类型名 |
| `<bundle>/type_map.toml` | 可选；配合 `ty_map` helper 做类型映射 |
| `.crudignore` | 排除不参与生成的模板 |

路径前缀（如 `java/`、`vue/`、`resources/`、`ddl/`、`sql/`）通过 `.crud/setup.toml` 的 `[paths.lang]` / `[paths.aux]` 映射到宿主项目目录。建表 DDL 建议放在 `ddl/`（便于 `--type ddl` / MCP `preview` 单独预览）；菜单或数据类 SQL 放在 `sql/`；两者可映射到同一物理目录。详见 [README.zh.md](../../README.zh.md#路径系统)。

---

## Front-matter

任意 `.hbs` 文件顶部可选 YAML 块（以 `---` 包裹）：

```yaml
---
basePath: "java/{{package_path}}/service/impl"
filename: "{{model_pascal}}ServiceImpl.java"
overwrite: force-only          # never | force-only | always
generateWhen: has_import       # 与 skipWhen 二选一
---
```

| 键 | 说明 |
|----|------|
| `basePath` / `base_path` | 输出目录（相对项目根）；可引用内置或扩展顶层变量 |
| `filename` | 输出文件名（**单段**，不能含 `/`）；同样可引用变量 |
| `overwrite` | 覆盖策略，覆盖全局与用户默认 |
| `generateWhen` / `generate_when` | 条件为真时才生成此文件；值为 `{{#if ...}}` **内部**表达式（不带 `{{ }}`） |
| `skipWhen` / `skip_when` | 条件为真时**跳过**此文件；与 `generateWhen` 互斥 |

条件求值遵循 Handlebars 真值规则：`false`、缺失、空串、`0`、空数组均为假。

```yaml
---
generateWhen: has_import
filename: "{{model_pascal}}ImportDTO.java"
---
```

```yaml
---
skipWhen: "(eq mode \"slim\")"
filename: "{{model_pascal}}Service.java"
---
```

被条件跳过的文件在 `gen` 输出中标记为 `[skipped: condition]`。拼错的变量在生成时会被当作假而**静默跳过**，务必先运行 `crud-cli validate`。

---

## 内置顶层变量

下列变量由 `crud-cli` 在每次 `gen` 时**自动注入**渲染上下文，键名固定存在（值随本次生成参数变化）。

**不要**在 `_variables.toml` 或 `setup.toml` 的 `[variables]` 中声明与下列同名的键，否则会报 `variable shadows built-in`。

### 实体与表

| 变量 | 类型 | 说明 |
|------|------|------|
| `model` | string | 实体/类名（原始值，如 `User`） |
| `model_pascal` | string | PascalCase，如 `User` |
| `model_snake` | string | snake_case，如 `user` |
| `model_camel` | string | camelCase，如 `user` |
| `model_kebab` | string | kebab-case，如 `user` |
| `table` | string | 主表物理表名 |
| `table_comment` | string | 主表业务说明；未提供时为 `""` |
| `package` | string | 服务端包名（如 Java package） |
| `package_path` | string | `package` 中 `.` 替换为 `/`，如 `com/acme/demo` |

### 主键（由主表字段推导）

| 变量 | 类型 | 说明 |
|------|------|------|
| `pk_field` | string | 主键字段 camelCase 名 |
| `pk_field_type` | string | 主键字段原始类型字符串 |
| `pk_field_pascal` | string | 主键字段 PascalCase 名 |

若主表 `fields` 中没有任何 `is_pk: true` 的字段，默认值为 `id` / `Long` / `Id`。

### 主子表

| 变量 | 类型 | 无主子表时 |
|------|------|------------|
| `is_sub` | bool | `false` |
| `sub_table` | string | `""` |
| `sub_table_comment` | string | `""` |
| `sub_fields` | array | `[]` |
| `sub_model` | string | `""` |
| `sub_model_snake` | string | `""` |
| `sub_model_pascal` | string | `""` |
| `sub_model_camel` | string | `""` |
| `sub_model_kebab` | string | `""` |
| `sub_model_fk` | string | `""` |
| `sub_model_fk_pascal` | string | `""` |

存在主子表关系时，`is_sub` 为 `true`，其余字段由子实体与外键列名填充。`sub_model_fk` 为外键列 camelCase；`sub_model_fk_pascal` 常用于 Java setter 名。

### 字段列表

| 变量 | 类型 | 说明 |
|------|------|------|
| `fields` | array | 主表字段数组 |
| `sub_fields` | array | 子表字段数组；无主子表时为 `[]` |

在模板中用 `{{#each fields}}` / `{{#each sub_fields}}` 遍历；每一项的属性见下一节。

### 作者与时间

| 变量 | 类型 | 说明 |
|------|------|------|
| `git_user_name` | string | 来自 git config |
| `git_user_email` | string | 来自 git config |
| `user_name` | string | 来自 `.crud/setup.user.toml`；为空则回退 `git_user_name` |
| `user_email` | string | 来自 `.crud/setup.user.toml`；为空则回退 `git_user_email` |
| `date` | string | 本地日期，`YYYY-MM-DD` |
| `datetime` | string | 本地日期时间，`YYYY-MM-DD HH:MM:SS` |
| `year` | string | 四位年份 |

### Handlebars 上下文特殊名

在 `{{#each}}` 块内还可使用（由 Handlebars 提供，校验器视为合法）：

| 变量 | 说明 |
|------|------|
| `this` | 当前迭代项 |
| `@index` | 从 0 开始的索引 |
| `@key` | 对象迭代时的键 |
| `@first` / `@last` | 是否首项 / 末项 |
| `@root` | 根上下文 |

---

## 字段对象（`{{#each fields}}`）

`fields` / `sub_fields` 数组中，**每一项**在模板里暴露下列默认属性：

| 属性 | 类型 | 说明 |
|------|------|------|
| `name` | string | 列名（原始值） |
| `name_pascal` | string | PascalCase |
| `name_snake` | string | snake_case |
| `name_camel` | string | camelCase |
| `name_kebab` | string | kebab-case |
| `type` | string | 字段类型字符串（与 `_field_types.toml` 中的 canonical 名一致） |
| `is_pk` | bool | 是否主键 |
| `nullable` | bool | 是否可空 |
| `comment` | string | 注释/文案；未提供时为 `""` |
| `length` | number \| null | 长度；未提供时为 `null` |
| `unique` | bool | 是否唯一；未提供时为 `false` |
| `default` | any \| null | 默认值；未提供时为 `null` |

示例：

```handlebars
{{#each fields}}
  {{#if is_pk}}
    /** {{comment}} */
    private {{ty_map type}} {{name_camel}};
  {{/if}}
{{/each}}
```

使用 `--fields` DSL 快速生成时，通常只有 `name`、`type`、`is_pk`、`nullable` 有值；`comment` 为空串，`length` / `default` 为 `null`，`unique` 为 `false`。需要完整元数据（注释、长度、唯一约束等）时，应通过带字段明细的 gen 输入提供。

### 扩展字段属性

除上表默认键外，**可按模板包需要自行扩展**：调用方传入的额外键值会**扁平合并**进每个字段对象，在 `{{#each fields}}` 内与默认属性同级访问。

例如模板包约定扩展 `query`（bool）、`dict_type`（string）：

```handlebars
{{#each fields}}
  {{#if query}}
    <el-form-item label="{{comment}}">...</el-form-item>
  {{/if}}
{{/each}}
```

扩展键的语义由**模板包作者**定义，并在模板包说明或 `_field_types.toml` 旁文档中写清，供 Agent 在构造 gen 命令时填入对应值。`validate` 的静态变量检查以默认属性名为准；引用扩展键时若出现 `unknown variable`，可确认键名拼写与模板包约定一致。

---

## 扩展顶层变量

内置顶层变量之外，可按需增加**自定义顶层变量**，供 front-matter、`{{#if}}` 与模板正文使用：

| 方式 | 说明 |
|------|------|
| `_variables.toml` | 在模板包根目录声明 schema（`type`、`default`、`required`、`description`）；`description` 供 Agent 理解用途 |
| `.crud/setup.toml` → `[variables]` | 项目级默认值，合并到顶层上下文 |
| `--var key=value` | 单次 gen 覆盖 |
| gen 输入中的 `variables` 对象 | 单次 gen 覆盖；与 `--var` 同义 |

优先级：`--var` > gen 输入 `variables` > schema `default`。

```toml
# _variables.toml 示例
[has_import]
description = "是否生成导入按钮和 importExcel 接口"
type        = "bool"
default     = false

[module_name]
description = "业务模块标识，用于权限前缀与路由"
type        = "string"
required    = true
```

自定义顶层变量**不得**与内置变量或 `fields` 等保留名冲突。`validate` 会检查模板引用的变量是否属于：**内置变量** ∪ **schema 声明** ∪ **`[variables]` 配置**。

---

## 内置 Helper

`crud-cli` 在 Handlebars 引擎上注册了下列 helper。此外，标准 Handlebars 块 helper（`if`、`unless`、`each`、`with` 等）与子表达式（如 `(eq a b)`）均可正常使用。

### 命名风格转换

均接受一个字符串参数，返回转换结果：

| Helper | 示例输入 | 输出 |
|--------|----------|------|
| `pascal_case` | `hello_world` | `HelloWorld` |
| `snake_case` | `HelloWorld` | `hello_world` |
| `camel_case` | `hello_world` | `helloWorld` |
| `kebab_case` | `hello_world` | `hello-world` |

```handlebars
{{pascal_case model_snake}}
{{camel_case "order_item_id"}}
```

### 大括号包装（MyBatis / Vue 占位符）

模板引擎**不做 HTML 转义**，Java 泛型等可原样输出。下列 helper 用于在生成结果中**嵌入**一层或两层字面量大括号，避免与 Handlebars 自身语法冲突：

| Helper | 示例 | 输出 |
|--------|------|------|
| `single_brace` | `{{single_brace name_camel}}`（上下文 `name_camel=userId`） | `{userId}` |
| `double_brace` | `{{double_brace name_camel}}` | `{{userName}}` |

MyBatis 常见写法（`#` / `$` 前缀写在 helper **外面**）：

```handlebars
WHERE id = #{{single_brace pk_field}}
ORDER BY ${{single_brace pk_field}}
```

Vue 模板中需要输出 `{{变量}}` 字面量时：

```handlebars
<span>{{double_brace name_camel}}</span>
```

### 类型映射 `ty_map`

将 neutral 类型名映射为目标栈类型（如 Java `Integer`、TS `number`）：

```handlebars
private {{ty_map type}} {{name_camel}};
```

映射表来自当前 bundle 下的 `type_map.toml`；未命中时行为由 `.crud/setup.toml` → `[type_map].fallback` 决定：

| fallback | 行为 |
|----------|------|
| `passthrough`（默认） | 原样输出类型字符串 |
| `error` | 渲染失败 |
| 任意其他字符串 | 作为固定字面量替换 |

### Handlebars 标准能力（未单独注册）

下列由 Handlebars 内置提供，`validate` 不会报 `missing_helper`：

- **块 helper：** `{{#if}}` / `{{#unless}}` / `{{#each}}` / `{{#with}}`
- **子表达式：** `(eq a b)`、`(ne a b)`、`(and a b)`、`(or a b)`、`(not x)` 等，常用于 front-matter 条件或复杂分支
- **路径：** `../` 访问父级上下文；`lookup` 动态取属性

---

## Agent 编写模板前应读取的目标项目信息

模板的目标是让生成代码与**宿主项目逐字节一致**。Agent 在新建或改造模板包之前，应系统阅读目标仓库，而不是套用通用脚手架。下列清单按技术栈归纳——实际项目可能只涉及其中一部分，按需裁剪即可。

### 所有项目通用

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 目录布局 | 源码、资源、测试、文档、前端各自根路径；是否 monorepo / 多模块 |
| `.crud/setup.toml` | `[project]`、`[paths.lang]`、`[paths.aux]`、`[type_map]`、`[variables]` 是否已配置 |
| 既有 CRUD 样例 | 找 1～2 个与待生成功能同类的**手工编写**文件，作为模板的「金标准」 |
| 命名约定 | 类/文件/表/列/API 路径的大小写与前后缀（如 `XxxController`、`sys_` 表前缀） |
| 注释与文件头 | 是否要求 `@author`、版权块、生成日期（可用内置 `user_name`、`date`） |
| 权限与安全 | 注解、中间件、路由 guard 的写法与命名 |
| 错误与响应格式 | 统一返回体、错误码、分页结构 |
| 日志与审计 | 使用的 logger、操作日志是否需模板化 |
| 测试位置与风格 | 测试目录、基类、Mock 方式 |

### Java / Kotlin（Spring、MyBatis、JPA 等）

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 包结构 | `controller` / `service` / `mapper` / `domain` / `dto` / `vo` 分层与包名规则 |
| Web 层 | `@RestController` 路径前缀、HTTP 动词、参数注解（`@RequestBody`、`@PathVariable`） |
| 统一响应 | `R`、`AjaxResult`、`Result` 等封装类及静态工厂方法名 |
| 异常体系 | 业务异常基类、全局 `@ControllerAdvice`、错误码枚举 |
| 校验 | `javax` / `jakarta.validation` 注解习惯、`@Validated` 分组 |
| 持久层 | MyBatis XML 还是注解；`#{}` / `${}` 习惯；主键策略与逻辑删除字段 |
| ORM 实体 | 基类（`BaseEntity`）、Lombok 注解组合、表字段映射与 `@TableLogic` |
| 分页 | `PageHelper`、`IPage`、请求/响应 DTO 字段名 |
| 导入导出 | 若项目有 Excel 模块，DTO 与 Controller 方法签名 |
| 事务与权限 | `@Transactional` 位置、`@PreAuthorize` / 自定义权限字符串格式 |

### TypeScript / JavaScript

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 运行时框架 | NestJS 模块/DTO/装饰器，或 Express/Fastify 路由与中间件 |
| 校验 | `class-validator`、Zod、Joi 等及错误抛出方式 |
| ORM | Prisma schema 命名、TypeORM 实体装饰器、Sequelize 模型 |
| API 层 | Controller/Handler 返回类型、Interceptor、Exception Filter |
| 前端（若同仓） | 见下文 Vue/React |

### Go

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 包路径 | `internal/` 下按 feature 还是 layer 划分 |
| Web 框架 | Gin/Echo/Fiber 路由注册、handler 签名、中间件链 |
| 错误处理 | 自定义 `error` 类型、HTTP 状态码映射 |
| 数据访问 | GORM / sqlx 标签、Repository 接口位置 |
| 配置与 DI | wire/fx 等是否影响生成文件结构 |

### Python

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 框架 | Django apps/models/admin/serializers，或 FastAPI router/dependency |
| 模型 | Pydantic `BaseModel`、SQLAlchemy 声明式模型、Alembic 迁移习惯 |
| 校验与响应 | `HTTPException`、统一 response_model、分页 schema |
| 异步 | `async def` 是否为主流、session 生命周期 |

### C# / .NET

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 项目类型 | Web API、Minimal API、Clean Architecture 各层目录 |
| 数据注解 | FluentValidation vs DataAnnotations |
| EF Core | DbContext、实体配置、迁移命令 |
| 统一结果 | `ActionResult<T>`、ProblemDetails、自定义 `ApiResponse` |

### PHP

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 框架 | Laravel Controller/Request/Resource/Policy 约定，或 Symfony bundle 结构 |
| ORM | Eloquent 模型 trait、迁移文件命名 |
| 校验 | FormRequest、规则数组写法 |

### Rust

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| Web | axum / actix-web 路由与 extractors |
| 数据 | sqlx / diesel 模型与 migration 目录 |
| 错误 | `thiserror`、`anyhow`、IntoResponse 映射 |

### 前端（Vue / React）

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 目录 | views/pages、components、api、router、store 的实际路径 |
| API 客户端 | axios 封装、请求/响应类型、baseURL |
| 列表页模式 | 表格组件、搜索表单、分页参数名、权限指令（`v-hasPermi` 等） |
| 表单与校验 | 组件库（Element Plus、Ant Design）字段绑定与 rules |
| 路由 | 动态路由、meta 字段、懒加载写法 |
| 状态 | Pinia/Vuex/Redux 是否参与 CRUD 页 |

### 数据库与 SQL 模板

| 关注点 | 要弄清楚什么 |
|--------|--------------|
| 方言 | MySQL / PostgreSQL 类型与索引语法 |
| 命名 | 表前缀、列命名、字符集与引擎默认值 |
| 是否分离 DDL | 使用 `--stdout --type sql` 时 DDL 模板是否独立 bundle |

### 推荐工作流（Agent）

1. 阅读 `.crud/setup.toml` 与模板包内 `_variables.toml`、`_field_types.toml`。
2. 在目标项目中定位**同类功能的现有实现**（Controller + Service + 前端列表页各一份）。
3. 列出与通用脚手架的差异点（返回类型、基类、注解、路径前缀、权限字符串）。
4. 编写或修改 `.hbs`，用 front-matter 对齐输出路径；用 `_variables.toml` 声明扩展顶层变量，并在模板包说明中约定扩展字段属性（若有）。
5. 运行 `crud-cli validate`，再用 `--dry-run` / `--stdout` 对比生成结果与金标准。
6. 差异应通过**改模板**消化，而不是生成后再手工改代码。

---

## 引擎特性

- **无 HTML 转义：** `{{type}}` 等原样输出，`<List<T>>` 不会被破坏。
- **确定性校验：** `validate` 静态分析变量引用；条件 front-matter 中的变量也必须在 schema 或内置列表中。
- **事务写盘：** 任一输出文件冲突可能导致整批回滚（取决于 overwrite 策略）。

---

## 常见错误

| 提示 | 排查 |
|------|------|
| `unknown variable` / `UnknownVariable` | 模板引用了未在内置列表、`_variables.toml` 或 `[variables]` 中声明的变量 |
| `variable shadows built-in` | `_variables.toml` 或 `[variables]` 使用了内置变量名 |
| `missing_helper` | 拼错 helper 名；确认使用的是上文列出的 helper 或 Handlebars 内置 helper |
| `helper not found`（渲染阶段） | 同上；或 `ty_map` 在 fallback=error 时因未映射类型间接失败 |
| 文件被 `[skipped: condition]` | `generateWhen` / `skipWhen` 为假；检查变量值与拼写 |
| 条件静默跳过 | 未声明变量在条件中被当作假 — 务必先 `validate` |
| `invalid filename` | front-matter `filename` 含 `/` 或路径遍历片段 |
| front-matter YAML 解析失败 | 含 `{{` 的值需加引号 |

---

## 延伸阅读

- [README.zh.md](../../README.zh.md) — CLI、路径系统、安装模板包
- [文档索引](../README.md)
