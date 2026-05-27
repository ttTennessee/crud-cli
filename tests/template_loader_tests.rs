//! Template discovery under `.crud/templates/`.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::error::Kind;
use crud_cli::core::template_loader::discover_templates;
use std::fs;
use tempfile::TempDir;

#[test]
fn discover_templates_lists_hbs_files() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Entity.java.hbs"), "e").unwrap();
    fs::write(templates.join("Mapper.java.hbs"), "m").unwrap();

    let entries = discover_templates(root, None).expect("discover");
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].rel_path, std::path::PathBuf::from("Entity.java.hbs"));
    assert_eq!(entries[1].rel_path, std::path::PathBuf::from("Mapper.java.hbs"));
}

#[test]
fn crudignore_filters_templates() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    let templates = root.join(".crud/templates");
    fs::create_dir_all(&templates).unwrap();
    fs::write(templates.join("Entity.java.hbs"), "e").unwrap();
    fs::write(templates.join("Mapper.java.hbs"), "m").unwrap();
    fs::write(templates.join(".crudignore"), "Mapper.*\n").unwrap();

    let entries = discover_templates(root, None).expect("discover");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].rel_path, std::path::PathBuf::from("Entity.java.hbs"));
}

#[test]
fn empty_templates_dir_errors() {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path();
    fs::create_dir_all(root.join(".crud/templates")).unwrap();

    let err = discover_templates(root, None).expect_err("empty");
    assert_eq!(err.kind, Kind::UserError);
    assert_eq!(
        err.details.get("reason").and_then(|v| v.as_str()),
        Some("no_templates_found")
    );
}
