//! Framework path defaults (D-11 / CONF-05).

use super::config::{Backend, Frontend, PathsSection};

/// Applies locked path keys for the selected backend and frontend only.
#[must_use]
pub fn paths_for_frameworks(backend: Backend, frontend: Frontend) -> PathsSection {
    let mut paths = PathsSection::default();
    match backend {
        Backend::SpringBoot => paths.java_base = Some("src/main/java".to_string()),
        Backend::Nest => paths.nest_base = Some("src".to_string()),
        Backend::None => {}
    }
    match frontend {
        Frontend::Vue => paths.vue_base = Some("src/views".to_string()),
        Frontend::React => paths.react_base = Some("src/views".to_string()),
        Frontend::None => {}
    }
    paths
}
