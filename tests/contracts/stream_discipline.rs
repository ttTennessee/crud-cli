//! Stream discipline contract: `println!` / `eprintln!` are only permitted in
//! `src/cli/output.rs`. Everywhere else must funnel through the helpers there
//! so agent-mode JSON and human output share one formatting path, and so
//! `core` stays usable from the MCP server without leaking to real streams.
//!
//! Replaces the equivalent ripgrep step in `.github/workflows/ci.yml`, which
//! only ran on Linux. This test runs on every platform under `cargo test`.

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: &[&str] = &["src/cli/output.rs"];
const FORBIDDEN: &[&str] = &["println!", "eprintln!", "print!", "eprint!"];

#[test]
fn no_println_outside_output_rs() {
    let src = repo_root().join("src");
    let mut violations = Vec::new();
    walk(&src, &mut violations);
    assert!(
        violations.is_empty(),
        "stream discipline violated — {} macro call(s) outside src/cli/output.rs:\n{}",
        violations.len(),
        violations.join("\n"),
    );
}

fn walk(dir: &Path, out: &mut Vec<String>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
            continue;
        }
        if path.extension().and_then(|s| s.to_str()) != Some("rs") {
            continue;
        }
        if is_allowed(&path) {
            continue;
        }
        scan_file(&path, out);
    }
}

fn is_allowed(path: &Path) -> bool {
    let rel = path
        .strip_prefix(repo_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    ALLOWED.iter().any(|a| rel == *a)
}

fn scan_file(path: &Path, out: &mut Vec<String>) {
    let body = match fs::read_to_string(path) {
        Ok(b) => b,
        Err(_) => return,
    };
    let rel = path
        .strip_prefix(repo_root())
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    for (i, line) in body.lines().enumerate() {
        let stripped = strip_line_comment(line).trim_start();
        for needle in FORBIDDEN {
            if stripped.contains(needle) {
                out.push(format!("  {}:{}  {}", rel, i + 1, line.trim()));
                break;
            }
        }
    }
}

// Best-effort `// ...` comment stripper. Doesn't try to be a Rust parser; if a
// `println!` ever appears inside a `/* ... */` block comment or a string
// literal, accept the false positive — it's vanishingly rare and we'd rather
// over-warn than under-warn.
fn strip_line_comment(line: &str) -> &str {
    match line.find("//") {
        Some(i) => &line[..i],
        None => line,
    }
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
