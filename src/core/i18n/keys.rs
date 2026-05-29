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
    ERROR_TYPE_MAP_READ_FAILED,
    ERROR_TYPE_MAP_PARSE_FAILED,
    ERROR_TYPE_MAP_UNMAPPED_BUNDLE,
    ERROR_TYPE_MAP_UNMAPPED_GLOBAL,
    ERROR_TEMPLATE_TYPE_NOT_FOUND,
    ERROR_TEMPLATE_NO_TEMPLATES,
    ERROR_TEMPLATE_WALK_ERROR,
    ERROR_TEMPLATE_INVALID_TYPE_GLOB,
];
