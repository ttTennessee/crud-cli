# 文档

**Languages:** [English](../README.md) · 简体中文

人类向文档。给 LLM Agent 的机器可读 spec 位于 [`../../agent-resources/`](../../agent-resources/)（英文唯一）。

| 路径 | 受众 | 状态 |
|---|---|---|
| [quickstart.md](./quickstart.md) | 新用户 —— 安装、基本使用、CLI 子命令参考 | 简体中文 |
| [templates.md](./templates.md) | 模板作者 | 占位 —— 完整指南待补全 |
| [entity.md](./entity.md) | `entity.json` 规范 | 简体中文 |
| [mcp.md](./mcp.md) | MCP 集成方 —— server 配置、工具、资源、prompts | 简体中文 |
| [../dev/](../dev/) | 贡献者（`crud-cli` 开发） | 英文 |

## 机器可读 spec

下列文件在 `docs/` 之外，是因为它们通过 `include_str!` 嵌入二进制，并作为 MCP resources / prompts 提供给 LLM Agent。写作规范不同（精炼 spec、无人类向行文），仅维护英文版本。

| 路径 | 作为 |
|---|---|
| [../../agent-resources/template-authoring.md](../../agent-resources/template-authoring.md) | MCP prompt `crud_template_authoring` |
| [../../agent-resources/entity.md](../../agent-resources/entity.md) | MCP resource `crud://schema/entity` |
