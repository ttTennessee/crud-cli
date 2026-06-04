//! `ddl/` template prefix is distinct from `sql/` (menu/data SQL) for scoped preview.
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::template_loader::discover_templates;
use tempfile::tempdir;

#[test]
fn ddl_prefix_only_includes_schema_not_menu() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    std::fs::create_dir_all(root.join("ddl")).unwrap();
    std::fs::create_dir_all(root.join("sql")).unwrap();
    std::fs::write(root.join("ddl/schema.sql.hbs"), "-- ddl").unwrap();
    std::fs::write(root.join("sql/menu.sql.hbs"), "-- menu").unwrap();

    let ddl_only = discover_templates(root, Some(&["ddl".to_string()])).expect("discover ddl");
    assert!(
        ddl_only.iter().any(|e| {
            e.rel_path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("ddl/schema.sql.hbs")
        }),
        "expected ddl/schema.sql.hbs"
    );
    assert!(
        !ddl_only.iter().any(|e| {
            e.rel_path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("menu.sql")
        }),
        "menu SQL must not match ddl filter"
    );

    let sql_only = discover_templates(root, Some(&["sql".to_string()])).expect("discover sql");
    assert!(
        sql_only
            .iter()
            .any(|e| e.rel_path.to_string_lossy().contains("menu.sql")),
        "expected sql/menu.sql.hbs"
    );
    assert!(
        !sql_only.iter().any(|e| {
            e.rel_path
                .to_string_lossy()
                .replace('\\', "/")
                .contains("ddl/")
        }),
        "ddl templates must not match sql filter"
    );
}
