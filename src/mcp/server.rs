//! MCP server (`crud-cli mcp`): tools, resources, and prompts.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use rmcp::{
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    handler::server::wrapper::Parameters,
    model::{
        AnnotateAble, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        ListPromptsResult, ListResourceTemplatesResult, ListResourcesResult, PaginatedRequestParams,
        PromptMessage, PromptMessageRole, RawResource, ReadResourceRequestParams,
        ReadResourceResult, Resource, ResourceContents, ServerCapabilities, ServerInfo,
    },
    prompt, prompt_handler, prompt_router, tool, tool_handler, tool_router, ErrorData as McpError,
    RoleServer, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use rmcp::service::RequestContext;
use serde::Deserialize;
use serde_json::Value;

use crate::core::error::ErrorEnvelope;
use crate::core::gen_pipeline;
use crate::core::gen_run::GenRunParams;

use super::context::{load_project_context, ProjectContext};
use super::convert::{envelope_to_value, generate_report_value};
use super::resources::{list_static_resources, read_resource};
use super::validate_logic::{describe_templates, entity_json_to_temp_path, preview_entity_structure};

/**
 * MCP server state: tool/prompt routers plus lazily resolved project context.
 */
#[derive(Clone)]
pub struct CrudMcpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    #[allow(dead_code)]
    prompt_router: PromptRouter<Self>,
    project: Arc<std::sync::Mutex<Option<ProjectContext>>>,
}

impl CrudMcpServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            project: Arc::new(std::sync::Mutex::new(None)),
        }
    }

    fn with_project(
        &self,
        cwd: Option<PathBuf>,
    ) -> Result<ProjectContext, rmcp::model::ErrorData> {
        let mut guard = self
            .project
            .lock()
            .map_err(|_| internal_err("project lock poisoned"))?;
        if let Some(ref ctx) = *guard {
            if cwd.is_none() {
                return Ok(ctx.clone());
            }
        }
        let ctx = load_project_context(cwd).map_err(envelope_err)?;
        *guard = Some(ctx.clone());
        Ok(ctx)
    }

    fn templates_root(&self) -> Result<PathBuf, McpError> {
        let ctx = self.with_project(None)?;
        Ok(ctx.templates_root.clone())
    }
}

#[derive(Debug, Deserialize, JsonSchema)]
struct PreviewParams {
    /// Full entity.json document (UTF-8).
    #[schemars(description = "entity.json content")]
    entity_json: String,
    /// Optional JSON object of template variables (same keys as _variables.toml).
    #[schemars(description = "Optional variables object as JSON")]
    variables: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GenerateParams {
    #[schemars(description = "entity.json content")]
    entity_json: String,
    #[schemars(description = "Optional variables object as JSON")]
    variables: Option<Value>,
    #[schemars(description = "Optional type prefix filter (comma-separated)")]
    r#type: Option<String>,
    #[schemars(description = "Overwrite existing files when policy allows")]
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct DescribeParams {
    /// Optional project root; defaults to process cwd.
    #[schemars(description = "Project root directory")]
    project_root: Option<String>,
}

fn parse_type_filter(raw: Option<&str>) -> Option<Vec<String>> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .collect()
    })
}

fn vars_from_optional(value: Option<Value>) -> BTreeMap<String, Value> {
    match value {
        Some(Value::Object(map)) => map.into_iter().collect(),
        _ => BTreeMap::new(),
    }
}

fn tool_json_result(value: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(&e.to_string()))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/**
 * Returns markdown first (for direct user display), then JSON payload.
 */
fn tool_preview_result(value: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(&e.to_string()))?;
    let markdown = value
        .get("display_markdown")
        .or_else(|| value.get("table_markdown"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_default();
    if markdown.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(text)]));
    }
    Ok(CallToolResult::success(vec![
        Content::text(markdown),
        Content::text(text),
    ]))
}

fn tool_error_result(value: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(&e.to_string()))?;
    Ok(CallToolResult::error(vec![Content::text(text)]))
}

fn run_gen_blocking(
    ctx: ProjectContext,
    entity_json: String,
    cli_vars: BTreeMap<String, Value>,
    type_filter: Option<Vec<String>>,
    force: bool,
) -> Result<Value, ErrorEnvelope> {
    let temp = entity_json_to_temp_path(&entity_json)?;
    let params = GenRunParams {
        file: Some(temp.to_path_buf()),
        type_filter,
        stdout: false,
        force,
        cli_vars,
        ..GenRunParams::default()
    };
    std::env::set_current_dir(&ctx.cwd).map_err(|e| {
        ErrorEnvelope::user_error(
            format!("chdir: {e}"),
            None,
            None,
            "could not set project working directory",
        )
    })?;
    let report = gen_pipeline::run(params)?;
    Ok(generate_report_value(&report))
}

#[tool_router]
impl CrudMcpServer {
    /**
     * Returns variables/field-types schemas as JSON, type prefixes, and path mappings.
     */
    #[tool(description = "Describe the active template bundle for entity.json authoring")]
    async fn describe_templates(
        &self,
        Parameters(p): Parameters<DescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let cwd = p.project_root.map(PathBuf::from);
        let ctx = self.with_project(cwd)?;
        let value = tokio::task::spawn_blocking(move || describe_templates(&ctx))
            .await
            .map_err(|e| internal_err(&e.to_string()))?
            .map_err(envelope_err)?;
        tool_json_result(value)
    }

    /**
     * Validates entity.json and returns its normalized structure as a confirmation
     * table (no template code is rendered or written).
     */
    #[tool(
        description = "Validate entity.json and preview its normalized field structure as a table"
    )]
    async fn preview(
        &self,
        Parameters(p): Parameters<PreviewParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.with_project(None)?;
        let cli_vars = vars_from_optional(p.variables);
        let json = p.entity_json;
        let result =
            tokio::task::spawn_blocking(move || preview_entity_structure(&ctx, &json, &cli_vars))
                .await
                .map_err(|e| internal_err(&e.to_string()))?;
        match result {
            Ok(v) => tool_preview_result(v),
            Err(envelope) => tool_error_result(envelope_to_value(&envelope)),
        }
    }

    /**
     * Validates and writes generated files to the project tree.
     */
    #[tool(description = "Generate CRUD files from entity.json")]
    async fn generate(
        &self,
        Parameters(p): Parameters<GenerateParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.with_project(None)?;
        let cli_vars = vars_from_optional(p.variables);
        let type_filter = parse_type_filter(p.r#type.as_deref());
        let json = p.entity_json;
        let force = p.force;
        let result = tokio::task::spawn_blocking(move || {
            run_gen_blocking(ctx, json, cli_vars, type_filter, force)
        })
        .await
        .map_err(|e| internal_err(&e.to_string()))?;
        match result {
            Ok(v) => tool_json_result(v),
            Err(envelope) => tool_error_result(envelope_to_value(&envelope)),
        }
    }
}

#[prompt_router]
impl CrudMcpServer {
    /**
     * Delivers the template authoring guide as a user message for the agent.
     */
    #[prompt(
        name = "template_authoring",
        description = "Guide for writing crud-cli Handlebars template bundles"
    )]
    async fn template_authoring_prompt(&self) -> GetPromptResult {
        let body = include_str!("../../docs/zh-CN/template-authoring.md");
        GetPromptResult::new(vec![PromptMessage::new_text(
            PromptMessageRole::User,
            format!(
                "Follow this crud-cli template authoring guide when creating or adapting a template bundle:\n\n{body}"
            ),
        )])
        .with_description(
            "crud-cli template authoring: structure, front-matter, helpers, schemas",
        )
    }
}

#[tool_handler]
#[prompt_handler]
impl ServerHandler for CrudMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(
            ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
        )
        .with_instructions(
            "crud-cli MCP: read crud:// resources for schemas, call describe_templates, \
             then preview (validates entity.json and returns its normalized field table \
             for user confirmation), then generate.",
        )
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let resources: Vec<Resource> = list_static_resources()
            .into_iter()
            .map(|(uri, name)| RawResource::new(uri, name).no_annotation())
            .collect();
        Ok(ListResourcesResult {
            resources,
            next_cursor: None,
            meta: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<ReadResourceResult, McpError> {
        let root = self.templates_root()?;
        let text = read_resource(&request.uri, &root).map_err(|msg| {
            McpError::resource_not_found("resource_not_found", Some(serde_json::json!({ "msg": msg })))
        })?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            text,
            &request.uri,
        )]))
    }

    async fn list_resource_templates(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourceTemplatesResult, McpError> {
        Ok(ListResourceTemplatesResult {
            resource_templates: vec![],
            next_cursor: None,
            meta: None,
        })
    }
}

fn internal_err(msg: impl Into<String>) -> McpError {
    McpError::internal_error(msg.into(), None)
}

fn envelope_err(e: ErrorEnvelope) -> McpError {
    McpError::invalid_params(
        e.msg,
        Some(serde_json::json!({
            "kind": e.kind,
            "hint": e.hint,
            "details": e.details,
        })),
    )
}

/**
 * Starts the MCP server on stdio (blocking until the client disconnects).
 */
pub async fn run_stdio_server() -> Result<(), anyhow::Error> {
    let server = CrudMcpServer::new();
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .map_err(|e| anyhow::anyhow!("mcp serve: {e}"))?;
    service
        .waiting()
        .await
        .map_err(|e| anyhow::anyhow!("mcp waiting: {e}"))?;
    Ok(())
}
