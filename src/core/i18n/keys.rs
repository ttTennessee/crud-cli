//! Centralized i18n message keys (A+ convention).
//!
//! Key naming: `<domain>.<subject>.<detail>`, lowercase `snake_case` segments,
//! dot-separated. Placeholders use named braces, e.g. `{count}`, `{path}`.
//! Every constant here MUST have an entry in both `en.toml` and `zh.toml`
//! (enforced by [`super::tests`]).

// ── setup command (interactive, human-only) ─────────────────────────────────
pub const SETUP_CREATED: &str = "setup.created";
pub const SETUP_CREATED_NO_PROJECT_HINT: &str = "setup.created_no_project_hint";
pub const SETUP_OVERWRITE_CONFIRM: &str = "setup.overwrite_confirm";

// ── setup wizard (interactive help / labels) ────────────────────────────────
pub const WIZARD_HELP_BACKEND: &str = "wizard.help.backend";
pub const WIZARD_HELP_ENABLED_TYPES: &str = "wizard.help.enabled_types";
pub const WIZARD_HELP_NAME: &str = "wizard.help.name";
pub const WIZARD_HELP_EMAIL: &str = "wizard.help.email";
pub const WIZARD_PATHS_CUSTOMIZE: &str = "wizard.paths.customize";
pub const WIZARD_PATHS_NONE: &str = "wizard.paths.none";
pub const WIZARD_PATHS_DEFAULTS: &str = "wizard.paths.defaults";
pub const WIZARD_TYPEMAP_PASSTHROUGH: &str = "wizard.typemap.passthrough";
pub const WIZARD_TYPEMAP_ERROR: &str = "wizard.typemap.error";
pub const WIZARD_TYPEMAP_LITERAL: &str = "wizard.typemap.literal";
pub const WIZARD_TYPEMAP_HELP: &str = "wizard.typemap.help";
pub const WIZARD_TYPEMAP_LITERAL_HELP: &str = "wizard.typemap.literal_help";
pub const WIZARD_NOT_EMPTY: &str = "wizard.not_empty";
pub const WIZARD_CANCELLED_MSG: &str = "wizard.cancelled_msg";
pub const WIZARD_CANCELLED_HINT: &str = "wizard.cancelled_hint";
pub const WIZARD_INVALID_SELECTION_MSG: &str = "wizard.invalid_selection_msg";
pub const WIZARD_INVALID_SELECTION_HINT: &str = "wizard.invalid_selection_hint";

// ── template wizard / subcommand ────────────────────────────────────────────
pub const WIZARD_TEMPLATE_DETECTED_HEADER: &str = "wizard.template.detected_header";
pub const WIZARD_TEMPLATE_MANUAL_OPTION: &str = "wizard.template.manual_option";
pub const WIZARD_TEMPLATE_NO_TEMPLATES: &str = "wizard.template.no_templates";
pub const WIZARD_TEMPLATE_CHOOSE_BACKEND: &str = "wizard.template.choose_backend";
pub const WIZARD_TEMPLATE_CHOOSE_FRONTEND: &str = "wizard.template.choose_frontend";
pub const WIZARD_TEMPLATE_CUSTOM_INPUT: &str = "wizard.template.custom_input";
pub const WIZARD_TEMPLATE_INVALID_LANG_NAME: &str = "wizard.template.invalid_lang_name";
pub const TEMPLATE_USE_CONFIRM: &str = "template.use.confirm";
pub const TEMPLATE_USE_APPLIED: &str = "template.use.applied";
pub const TEMPLATE_USE_MISSING_SETUP: &str = "template.use.missing_setup";
pub const TEMPLATE_LIST_EMPTY: &str = "template.list.empty";
pub const TEMPLATE_LIST_ENTRY: &str = "template.list.entry";
pub const TEMPLATE_INSTALL_STARTING: &str = "template.install.starting";
pub const TEMPLATE_INSTALL_DONE: &str = "template.install.done";
pub const TEMPLATE_INSTALL_PROMPT_FETCH: &str = "template.install.prompt_fetch";
pub const TEMPLATE_INSTALL_PROMPT_NAME: &str = "template.install.prompt_name";
pub const TEMPLATE_INSTALL_PROMPT_VERSION: &str = "template.install.prompt_version";
pub const TEMPLATE_INSTALL_PROMPT_DOC: &str = "template.install.prompt_doc";
pub const TEMPLATE_INSTALL_PROMPT_SQL: &str = "template.install.prompt_sql";
pub const TEMPLATE_INSTALL_PROMPT_DDL: &str = "template.install.prompt_ddl";
pub const TEMPLATE_INSTALL_BUNDLE_NONE: &str = "template.install.bundle_none";
pub const TEMPLATE_INSTALL_CONFIRM_OVERWRITE: &str = "template.install.confirm_overwrite";
pub const TEMPLATE_INSTALL_STATUS_INSTALLED: &str = "template.install.status_installed";
pub const TEMPLATE_INSTALL_STATUS_MODIFIED: &str = "template.install.status_modified";
pub const TEMPLATE_INSTALL_STATUS_OUTDATED: &str = "template.install.status_outdated";
pub const ERROR_TEMPLATE_INSTALL_NEEDS_TTY: &str = "error.template.install_needs_tty";
pub const ERROR_TEMPLATE_INSTALL_REPO_EMPTY: &str = "error.template.install_repo_empty";
pub const ERROR_CONFIG_LEGACY_SCHEMA: &str = "error.config.legacy_schema";

// ── gen / validate success lines (human-only) ───────────────────────────────
pub const GEN_SUCCESS_WRITTEN: &str = "gen.success.written";
pub const GEN_SUCCESS_DRY_RUN: &str = "gen.success.dry_run";
pub const VALIDATE_SUCCESS: &str = "validate.success";

// ── field DSL error hints ───────────────────────────────────────────────────
pub const ERROR_FIELD_EMPTY_TYPE: &str = "error.field.empty_type";
pub const ERROR_FIELD_EMPTY_NAME: &str = "error.field.empty_name";
pub const ERROR_FIELD_INVALID_IDENTIFIER: &str = "error.field.invalid_identifier";
pub const ERROR_FIELD_TOO_MANY_SEGMENTS: &str = "error.field.too_many_segments";
pub const ERROR_FIELD_DUPLICATE: &str = "error.field.duplicate";
pub const ERROR_FIELD_NO_FIELDS: &str = "error.field.no_fields";
pub const ERROR_FIELD_RESERVED: &str = "error.field.reserved";

// ── field extra schema error hints ──────────────────────────────────────────
pub const ERROR_FIELD_EXTRA_SCHEMA_FIX: &str = "error.field_extra.schema_fix";
pub const ERROR_FIELD_EXTRA_UNKNOWN_KEY: &str = "error.field_extra.unknown_key";
pub const ERROR_FIELD_EXTRA_MISSING_REQUIRED: &str = "error.field_extra.missing_required";

// ── field type schema error hints ───────────────────────────────────────────
pub const ERROR_FIELD_TYPE_UNSUPPORTED: &str = "error.field_type.unsupported";
pub const ERROR_FIELD_TYPE_UNSUPPORTED_DID_YOU_MEAN: &str =
    "error.field_type.unsupported_did_you_mean";
pub const ERROR_FIELD_TYPE_SCHEMA_FIX: &str = "error.field_type.schema_fix";
pub const ERROR_FIELD_TYPE_UNMAPPED_IN_BUNDLES: &str = "error.field_type.unmapped_in_bundles";

// ── type_map error hints ────────────────────────────────────────────────────
pub const ERROR_TYPE_MAP_READ_FAILED: &str = "error.type_map.read_failed";
pub const ERROR_TYPE_MAP_PARSE_FAILED: &str = "error.type_map.parse_failed";
pub const ERROR_TYPE_MAP_UNMAPPED_BUNDLE: &str = "error.type_map.unmapped_bundle";
pub const ERROR_TYPE_MAP_UNMAPPED_GLOBAL: &str = "error.type_map.unmapped_global";

// ── template loader error hints ─────────────────────────────────────────────
pub const ERROR_TEMPLATE_TYPE_NOT_FOUND: &str = "error.template.type_not_found";
pub const ERROR_TEMPLATE_NO_TEMPLATES: &str = "error.template.no_templates";
pub const ERROR_TEMPLATE_WALK_ERROR: &str = "error.template.walk_error";
pub const ERROR_TEMPLATE_INVALID_TYPE_GLOB: &str = "error.template.invalid_type_glob";

// ── file / path / global config error hints ─────────────────────────────────
pub const ERROR_FILE_CONFLICT: &str = "error.file.conflict";
pub const ERROR_PATHS_HOME_NOT_FOUND: &str = "error.paths.home_not_found";
pub const ERROR_PATHS_GITIGNORE_WRITE: &str = "error.paths.gitignore_write";
pub const ERROR_GLOBAL_CONFIG_CHECK: &str = "error.global_config.check";

// ── validate aggregated issues ──────────────────────────────────────────────
pub const ERROR_VALIDATE_FIX_ISSUES: &str = "error.validate.fix_issues";
pub const ERROR_VALIDATE_CWD: &str = "error.validate.cwd";

// ── JSON / --file input error hints ─────────────────────────────────────────
pub const ERROR_JSON_FILE_NOT_FOUND: &str = "error.json.file_not_found";
pub const ERROR_JSON_INVALID_SYNTAX: &str = "error.json.invalid_syntax";
pub const ERROR_JSON_NO_FIELDS: &str = "error.json.no_fields";
pub const ERROR_JSON_MISSING_NAME: &str = "error.json.missing_name";
pub const ERROR_JSON_MISSING_PACKAGE: &str = "error.json.missing_package";
pub const ERROR_JSON_MISSING_TABLE: &str = "error.json.missing_table";
pub const ERROR_JSON_MISSING_PROPERTY: &str = "error.json.missing_property";
pub const ERROR_JSON_TYPE_MISMATCH: &str = "error.json.type_mismatch";
pub const ERROR_JSON_INVALID_VALUE: &str = "error.json.invalid_value";
pub const ERROR_JSON_UNKNOWN_FIELD: &str = "error.json.unknown_field";
pub const ERROR_JSON_DID_YOU_MEAN: &str = "error.json.did_you_mean";

// ── generation pipeline error hints ─────────────────────────────────────────
pub const ERROR_GEN_FILENAME_SLASH: &str = "error.gen.filename_slash";
pub const ERROR_GEN_PATH_TRAVERSAL: &str = "error.gen.path_traversal";
pub const ERROR_GEN_CWD: &str = "error.gen.cwd";
pub const ERROR_GEN_MISSING_FLAG: &str = "error.gen.missing_flag";
pub const ERROR_GEN_MISSING_INPUT: &str = "error.gen.missing_input";

// ── CLI / args error hints ────────────────────────────────────────────────────
pub const ERROR_CLI_FIELDS_FILE_MUTEX: &str = "error.cli.fields_file_mutex";
pub const ERROR_CLI_CLAP_RETRY: &str = "error.cli.clap_retry";
pub const ERROR_CLI_MISSING_FLAG: &str = "error.cli.missing_flag";
pub const ERROR_CLI_EMPTY_FLAG: &str = "error.cli.empty_flag";

// ── setup command error hints ───────────────────────────────────────────────
pub const ERROR_SETUP_EXISTS_NON_INTERACTIVE: &str = "error.setup.exists_non_interactive";

// ── setup.toml config error hints ───────────────────────────────────────────
pub const ERROR_CONFIG_RESERVED_VARIABLE: &str = "error.config.reserved_variable";

// ── template variable error hints ───────────────────────────────────────────
pub const ERROR_VARIABLE_SHADOWS_BUILTIN: &str = "error.variable.shadows_builtin";
pub const ERROR_VARIABLE_UNDECLARED: &str = "error.variable.undeclared";
pub const ERROR_VARIABLE_MISSING_REQUIRED: &str = "error.variable.missing_required";
pub const ERROR_VARIABLE_TYPE_MISMATCH: &str = "error.variable.type_mismatch";
pub const ERROR_VARIABLE_SCHEMA_FIX: &str = "error.variable.schema_fix";
pub const ERROR_VARIABLE_INVALID_VAR_ARG: &str = "error.variable.invalid_var_arg";

/// Every key referenced from code; the consistency test asserts each one has a
/// catalog entry in both locales.
pub const ALL_KEYS: &[&str] = &[
    SETUP_CREATED,
    SETUP_CREATED_NO_PROJECT_HINT,
    SETUP_OVERWRITE_CONFIRM,
    WIZARD_HELP_BACKEND,
    WIZARD_HELP_ENABLED_TYPES,
    WIZARD_HELP_NAME,
    WIZARD_HELP_EMAIL,
    WIZARD_PATHS_CUSTOMIZE,
    WIZARD_PATHS_NONE,
    WIZARD_PATHS_DEFAULTS,
    WIZARD_TYPEMAP_PASSTHROUGH,
    WIZARD_TYPEMAP_ERROR,
    WIZARD_TYPEMAP_LITERAL,
    WIZARD_TYPEMAP_HELP,
    WIZARD_TYPEMAP_LITERAL_HELP,
    WIZARD_NOT_EMPTY,
    WIZARD_CANCELLED_MSG,
    WIZARD_CANCELLED_HINT,
    WIZARD_INVALID_SELECTION_MSG,
    WIZARD_INVALID_SELECTION_HINT,
    WIZARD_TEMPLATE_DETECTED_HEADER,
    WIZARD_TEMPLATE_MANUAL_OPTION,
    WIZARD_TEMPLATE_NO_TEMPLATES,
    WIZARD_TEMPLATE_CHOOSE_BACKEND,
    WIZARD_TEMPLATE_CHOOSE_FRONTEND,
    WIZARD_TEMPLATE_CUSTOM_INPUT,
    WIZARD_TEMPLATE_INVALID_LANG_NAME,
    TEMPLATE_USE_CONFIRM,
    TEMPLATE_USE_APPLIED,
    TEMPLATE_USE_MISSING_SETUP,
    TEMPLATE_LIST_EMPTY,
    TEMPLATE_LIST_ENTRY,
    TEMPLATE_INSTALL_STARTING,
    TEMPLATE_INSTALL_DONE,
    TEMPLATE_INSTALL_PROMPT_FETCH,
    TEMPLATE_INSTALL_PROMPT_NAME,
    TEMPLATE_INSTALL_PROMPT_VERSION,
    TEMPLATE_INSTALL_PROMPT_DOC,
    TEMPLATE_INSTALL_PROMPT_SQL,
    TEMPLATE_INSTALL_PROMPT_DDL,
    TEMPLATE_INSTALL_BUNDLE_NONE,
    TEMPLATE_INSTALL_CONFIRM_OVERWRITE,
    TEMPLATE_INSTALL_STATUS_INSTALLED,
    TEMPLATE_INSTALL_STATUS_MODIFIED,
    TEMPLATE_INSTALL_STATUS_OUTDATED,
    ERROR_TEMPLATE_INSTALL_NEEDS_TTY,
    ERROR_TEMPLATE_INSTALL_REPO_EMPTY,
    ERROR_CONFIG_LEGACY_SCHEMA,
    GEN_SUCCESS_WRITTEN,
    GEN_SUCCESS_DRY_RUN,
    VALIDATE_SUCCESS,
    ERROR_FIELD_EMPTY_TYPE,
    ERROR_FIELD_EMPTY_NAME,
    ERROR_FIELD_INVALID_IDENTIFIER,
    ERROR_FIELD_TOO_MANY_SEGMENTS,
    ERROR_FIELD_DUPLICATE,
    ERROR_FIELD_NO_FIELDS,
    ERROR_FIELD_RESERVED,
    ERROR_FIELD_EXTRA_SCHEMA_FIX,
    ERROR_FIELD_EXTRA_UNKNOWN_KEY,
    ERROR_FIELD_EXTRA_MISSING_REQUIRED,
    ERROR_FIELD_TYPE_UNSUPPORTED,
    ERROR_FIELD_TYPE_UNSUPPORTED_DID_YOU_MEAN,
    ERROR_FIELD_TYPE_SCHEMA_FIX,
    ERROR_FIELD_TYPE_UNMAPPED_IN_BUNDLES,
    ERROR_TYPE_MAP_READ_FAILED,
    ERROR_TYPE_MAP_PARSE_FAILED,
    ERROR_TYPE_MAP_UNMAPPED_BUNDLE,
    ERROR_TYPE_MAP_UNMAPPED_GLOBAL,
    ERROR_TEMPLATE_TYPE_NOT_FOUND,
    ERROR_TEMPLATE_NO_TEMPLATES,
    ERROR_TEMPLATE_WALK_ERROR,
    ERROR_TEMPLATE_INVALID_TYPE_GLOB,
    ERROR_FILE_CONFLICT,
    ERROR_PATHS_HOME_NOT_FOUND,
    ERROR_PATHS_GITIGNORE_WRITE,
    ERROR_GLOBAL_CONFIG_CHECK,
    ERROR_VALIDATE_FIX_ISSUES,
    ERROR_VALIDATE_CWD,
    ERROR_JSON_FILE_NOT_FOUND,
    ERROR_JSON_INVALID_SYNTAX,
    ERROR_JSON_NO_FIELDS,
    ERROR_JSON_MISSING_NAME,
    ERROR_JSON_MISSING_PACKAGE,
    ERROR_JSON_MISSING_TABLE,
    ERROR_JSON_MISSING_PROPERTY,
    ERROR_JSON_TYPE_MISMATCH,
    ERROR_JSON_INVALID_VALUE,
    ERROR_JSON_UNKNOWN_FIELD,
    ERROR_JSON_DID_YOU_MEAN,
    ERROR_GEN_FILENAME_SLASH,
    ERROR_GEN_PATH_TRAVERSAL,
    ERROR_GEN_CWD,
    ERROR_GEN_MISSING_FLAG,
    ERROR_GEN_MISSING_INPUT,
    ERROR_CLI_FIELDS_FILE_MUTEX,
    ERROR_CLI_CLAP_RETRY,
    ERROR_CLI_MISSING_FLAG,
    ERROR_CLI_EMPTY_FLAG,
    ERROR_SETUP_EXISTS_NON_INTERACTIVE,
    ERROR_CONFIG_RESERVED_VARIABLE,
    ERROR_VARIABLE_SHADOWS_BUILTIN,
    ERROR_VARIABLE_UNDECLARED,
    ERROR_VARIABLE_MISSING_REQUIRED,
    ERROR_VARIABLE_TYPE_MISMATCH,
    ERROR_VARIABLE_SCHEMA_FIX,
    ERROR_VARIABLE_INVALID_VAR_ARG,
];
