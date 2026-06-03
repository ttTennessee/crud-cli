//! Default `[paths]` map for a freshly-created project, derived from the
//! selected backend + frontend languages. Custom languages contribute no
//! defaults — the wizard prompts the user to fill them in.

use super::config::{Backend, Frontend, PathsSection};

/// Returns a `PathsSection` pre-populated with conventional defaults for the
/// chosen languages. `None` selections and `Custom(...)` languages add nothing.
#[must_use]
pub fn paths_for_selections(backend: &Backend, frontend: &Frontend) -> PathsSection {
    let mut paths = PathsSection::default();
    paths.aux.insert("sql".into(), "sql".into());
    // DDL templates use the `ddl/` prefix but land in the same dir as data SQL.
    paths.aux.insert("ddl".into(), "sql".into());
    match backend {
        Backend::Java => {
            paths.lang.insert("java".into(), "src/main/java".into());
            paths.aux.insert("resources".into(), "src/main/resources".into());
            paths.aux.insert("doc".into(), "doc/api".into());
        }
        Backend::TypeScript => {
            paths.lang.insert("ts".into(), "src".into());
            paths.aux.insert("doc".into(), "doc/api".into());
        }
        Backend::Go => {
            paths.lang.insert("go".into(), "internal".into());
            paths.aux.insert("doc".into(), "doc/api".into());
        }
        Backend::Python => {
            paths.lang.insert("python".into(), "src".into());
            paths.aux.insert("doc".into(), "doc/api".into());
        }
        Backend::None | Backend::Custom(_) => {}
    }
    match frontend {
        Frontend::Vue => {
            paths.lang.insert("vue".into(), "src/views".into());
        }
        Frontend::React => {
            paths.lang.insert("react".into(), "src/views".into());
        }
        Frontend::None | Frontend::Custom(_) => {}
    }
    paths
}
