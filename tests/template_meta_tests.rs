//! Front-matter parsing tests (D-G28 layer 1).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use crud_cli::core::config::OverwritePolicy;
use crud_cli::core::template_meta::split_front_matter;

#[test]
fn front_matter_parses_base_path_and_filename() {
    let src = "---\nbasePath: \"out/{{model_kebab}}\"\nfilename: \"{{model_pascal}}.java\"\noverwrite: never\n---\nbody line\n";
    let (meta, body) = split_front_matter(src).expect("ok");
    assert_eq!(meta.base_path.as_deref(), Some("out/{{model_kebab}}"));
    assert_eq!(meta.filename.as_deref(), Some("{{model_pascal}}.java"));
    assert_eq!(meta.overwrite, Some(OverwritePolicy::Never));
    assert!(body.contains("body line"));
    assert!(!body.starts_with("---"));
}

#[test]
fn no_front_matter_returns_defaults_and_full_body() {
    let src = "plain {{model}}\n";
    let (meta, body) = split_front_matter(src).expect("ok");
    assert!(meta.base_path.is_none());
    assert_eq!(body, src);
}

#[test]
fn front_matter_parses_generate_when_and_skip_when_aliases() {
    let camel = "---\ngenerateWhen: has_import\n---\nbody\n";
    let (meta, _) = split_front_matter(camel).expect("ok");
    assert_eq!(meta.generate_when.as_deref(), Some("has_import"));
    assert!(meta.skip_when.is_none());

    let snake = "---\nskip_when: \"(eq mode \\\"slim\\\")\"\n---\nbody\n";
    let (meta, _) = split_front_matter(snake).expect("ok");
    assert_eq!(meta.skip_when.as_deref(), Some("(eq mode \"slim\")"));
    assert!(meta.generate_when.is_none());
}

#[test]
fn front_matter_rejects_both_generate_when_and_skip_when() {
    let src = "---\ngenerateWhen: has_import\nskipWhen: legacy\n---\nbody\n";
    let result = split_front_matter(src);
    assert!(result.is_err(), "both conditions set must error");
}

#[test]
fn malformed_yaml_rejected() {
    let src = "---\n: invalid\n[\n---\nbody\n";
    let result = split_front_matter(src);
    assert!(
        result.is_err()
            || result
                .as_ref()
                .map(|(m, _)| m.base_path.is_none() && m.filename.is_none())
                .unwrap_or(false),
        "malformed front-matter should error or yield empty meta"
    );
}
