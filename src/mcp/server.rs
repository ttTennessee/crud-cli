//! MCP server (`crud-cli mcp`): tools, resources, and prompts.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rmcp::service::{NotificationContext, Peer, RequestContext};
use rmcp::{
    handler::server::router::{prompt::PromptRouter, tool::ToolRouter},
    handler::server::wrapper::Parameters,
    model::{
        AnnotateAble, CallToolResult, Content, GetPromptRequestParams, GetPromptResult,
        InitializeRequestParams, ListPromptsResult, ListResourceTemplatesResult,
        ListResourcesResult, PaginatedRequestParams, PromptMessage, PromptMessageRole, RawResource,
        ReadResourceRequestParams, ReadResourceResult, Resource, ResourceContents,
        ServerCapabilities, ServerInfo,
    },
    prompt, prompt_handler, prompt_router, tool, tool_handler, tool_router, ErrorData as McpError,
    RoleServer, ServerHandler, ServiceExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::{Mutex, RwLock};

use crate::core::error::ErrorEnvelope;
use crate::core::gen_pipeline;
use crate::core::gen_run::GenRunParams;

use super::context::{
    file_uri_to_path, load_project_context_from_start, ProjectContext, ROOTS_LIST_TIMEOUT,
};
use super::convert::{envelope_to_value, generate_report_value};
use super::resources::{list_resources, read_resource};
use super::validate_logic::{
    describe_templates, entity_json_to_temp_path, preview_entity_structure,
};

/**
 * MCP server state: tool/prompt routers plus lazily resolved project context.
 */
#[derive(Clone)]
pub struct CrudMcpServer {
    #[allow(dead_code)]
    tool_router: ToolRouter<Self>,
    #[allow(dead_code)]
    prompt_router: PromptRouter<Self>,
    explicit_path: Option<PathBuf>,
    supports_roots: Arc<AtomicBool>,
    resolved: Arc<RwLock<Option<Result<ProjectContext, ErrorEnvelope>>>>,
    resolve_lock: Arc<Mutex<()>>,
}

impl CrudMcpServer {
    fn new(explicit_path: Option<PathBuf>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            prompt_router: Self::prompt_router(),
            explicit_path,
            supports_roots: Arc::new(AtomicBool::new(false)),
            resolved: Arc::new(RwLock::new(None)),
            resolve_lock: Arc::new(Mutex::new(())),
        }
    }

    /**
     * Ensures the default project is loaded (scheme A: optional override only for describe).
     */
    async fn ensure_project(
        &self,
        override_root: Option<PathBuf>,
    ) -> Result<ProjectContext, McpError> {
        if let Some(start) = override_root {
            return load_project_context_from_start(start).map_err(envelope_err);
        }
        if let Some(ref cached) = *self.resolved.read().await {
            return cached.clone().map_err(envelope_err);
        }
        self.resolve_and_store(None, false).await;
        // Invariant: resolve_and_store always writes Some before returning.
        #[allow(clippy::expect_used)]
        self.resolved
            .read()
            .await
            .as_ref()
            .expect("resolve_and_store just wrote")
            .clone()
            .map_err(envelope_err)
    }

    async fn resolve_and_store(&self, peer: Option<&Peer<RoleServer>>, force: bool) {
        let _guard = self.resolve_lock.lock().await;
        if !force && self.resolved.read().await.is_some() {
            return;
        }

        let start = self.resolve_start_path(peer).await;
        let result =
            match tokio::task::spawn_blocking(move || load_project_context_from_start(start)).await
            {
                Ok(r) => r,
                Err(e) => Err(ErrorEnvelope::user_error(
                    format!("project resolve task: {e}"),
                    None,
                    None,
                    "internal error while loading the project",
                )),
            };
        *self.resolved.write().await = Some(result);
    }

    async fn resolve_start_path(&self, peer: Option<&Peer<RoleServer>>) -> PathBuf {
        if let Some(ref explicit) = self.explicit_path {
            return explicit.clone();
        }
        if self.supports_roots.load(Ordering::Relaxed) {
            if let Some(peer) = peer {
                match tokio::time::timeout(ROOTS_LIST_TIMEOUT, peer.list_roots()).await {
                    Ok(Ok(roots)) if !roots.roots.is_empty() => {
                        return file_uri_to_path(&roots.roots[0].uri);
                    }
                    Ok(Ok(_)) => {
                        tracing::warn!(
                            "roots/list returned no roots; falling back to process cwd."
                        );
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("roots/list failed ({e}); falling back to process cwd.");
                    }
                    Err(_) => {
                        tracing::warn!(
                            "roots/list timed out after {}s; falling back to process cwd.",
                            ROOTS_LIST_TIMEOUT.as_secs()
                        );
                    }
                }
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
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
    #[schemars(description = "entity_json content")]
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
    /// Optional project root; overrides the server default for this call only.
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
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(e.to_string()))?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

/**
 * Returns markdown first (for direct user display), then JSON payload.
 */
fn tool_preview_result(value: Value) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(e.to_string()))?;
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
    let text = serde_json::to_string_pretty(&value).map_err(|e| internal_err(e.to_string()))?;
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
            format!(
                "could not set working directory to project root {}",
                ctx.cwd.display()
            ),
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
    #[tool(
        name = "crud_describe_templates",
        description = "Describe the active template bundle for entity.json authoring"
    )]
    async fn describe_templates(
        &self,
        Parameters(p): Parameters<DescribeParams>,
    ) -> Result<CallToolResult, McpError> {
        let override_root = p.project_root.map(PathBuf::from);
        let ctx = self.ensure_project(override_root).await?;
        let value = tokio::task::spawn_blocking(move || describe_templates(&ctx))
            .await
            .map_err(|e| internal_err(e.to_string()))?
            .map_err(envelope_err)?;
        tool_json_result(value)
    }

    /**
     * Validates entity.json and returns its normalized structure as a confirmation
     * table (no template code is rendered or written).
     */
    #[tool(
        name = "crud_preview",
        description = "Validate entity.json and preview its normalized field structure as a table"
    )]
    async fn preview(
        &self,
        Parameters(p): Parameters<PreviewParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.ensure_project(None).await?;
        let cli_vars = vars_from_optional(p.variables);
        let json = p.entity_json;
        let result =
            tokio::task::spawn_blocking(move || preview_entity_structure(&ctx, &json, &cli_vars))
                .await
                .map_err(|e| internal_err(e.to_string()))?;
        match result {
            Ok(v) => tool_preview_result(v),
            Err(envelope) => tool_error_result(envelope_to_value(&envelope)),
        }
    }

    /**
     * Validates and writes generated files to the project tree.
     */
    #[tool(
        name = "crud_generate",
        description = "Generate CRUD files from entity.json"
    )]
    async fn generate(
        &self,
        Parameters(p): Parameters<GenerateParams>,
    ) -> Result<CallToolResult, McpError> {
        let ctx = self.ensure_project(None).await?;
        let cli_vars = vars_from_optional(p.variables);
        let type_filter = parse_type_filter(p.r#type.as_deref());
        let json = p.entity_json;
        let force = p.force;
        let result = tokio::task::spawn_blocking(move || {
            run_gen_blocking(ctx, json, cli_vars, type_filter, force)
        })
        .await
        .map_err(|e| internal_err(e.to_string()))?;
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
        name = "crud_template_authoring",
        description = "Guide for writing crud-cli Handlebars template bundles"
    )]
    async fn template_authoring_prompt(&self) -> GetPromptResult {
        let body = include_str!("../../agent-resources/template-authoring.md");
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
            "crud-cli MCP: call crud_describe_templates for the active bundle schemas, \
             read crud:// resources for entity.json docs, then crud_preview (validates \
             entity.json and returns its normalized field table for user confirmation), \
             then crud_generate. Prefer launching with `crud-cli mcp --path <project>` \
             or MCP roots when the process cwd is not the repo root.",
        )
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerInfo, McpError> {
        if context.peer.peer_info().is_none() {
            context.peer.set_peer_info(request.clone());
        }
        self.supports_roots
            .store(request.capabilities.roots.is_some(), Ordering::Relaxed);
        Ok(self.get_info())
    }

    async fn on_initialized(&self, context: NotificationContext<RoleServer>) {
        self.resolve_and_store(Some(&context.peer), false).await;
    }

    async fn on_roots_list_changed(&self, context: NotificationContext<RoleServer>) {
        *self.resolved.write().await = None;
        self.resolve_and_store(Some(&context.peer), true).await;
    }

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListResourcesResult, McpError> {
        let ctx = self.ensure_project(None).await?;
        let resources: Vec<Resource> = list_resources(&ctx.templates_root)
            .into_iter()
            .map(|(uri, name, description, mime)| {
                RawResource::new(uri, name)
                    .with_description(description)
                    .with_mime_type(mime)
                    .no_annotation()
            })
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
        let ctx = self.ensure_project(None).await?;
        let (text, mime) = read_resource(&request.uri, &ctx.templates_root).map_err(|msg| {
            McpError::resource_not_found(
                "resource_not_found",
                Some(serde_json::json!({ "msg": msg })),
            )
        })?;
        Ok(ReadResourceResult::new(vec![ResourceContents::text(
            text,
            &request.uri,
        )
        .with_mime_type(mime)]))
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
pub async fn run_stdio_server(explicit_path: Option<PathBuf>) -> Result<(), anyhow::Error> {
    let server = CrudMcpServer::new(explicit_path);
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
