//! Core library (no CLI dependencies).

pub mod config;
pub mod default_paths;
pub mod error;
pub mod field_dsl;
pub mod field_extra;
pub mod field_types;
pub mod fs_writer;
pub mod gen_context;
pub mod gen_input;
pub mod gen_pipeline;
pub mod gen_report;
pub mod gen_run;
pub mod git_info;
pub mod global_config;
pub mod i18n;
pub mod paths;
pub mod template_engine;
pub mod template_install_meta;
pub mod template_installer;
pub mod template_loader;
pub mod template_meta;
pub mod template_meta_global;
pub mod template_variables;
pub mod type_map;
pub mod validator;
