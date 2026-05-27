//! In-memory generation input (DSL and future JSON loader).

use serde::{Deserialize, Serialize};

use super::field_dsl::Field;

/// Entity input consumed by `build_context` and `gen_pipeline` (D-G13).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenInput {
    pub name: String,
    pub table: String,
    pub package: String,
    pub fields: Vec<Field>,
}
