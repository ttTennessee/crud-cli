//! GEN-02: `--file` and equivalent `--fields` produce byte-identical outputs (gap closure 02-06).
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use crud_cli::core::config::SetupConfig;
use crud_cli::core::config::SetupSelections;
use crud_cli::core::config::{Backend, Frontend};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
}

fn exe() -> String {
    std::env::var("CARGO_BIN_EXE_crud_cli").unwrap_or_else(|_| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("target/debug/crud-cli")
            .to_string_lossy()
            .into_owned()
    })
}

fn seed_gen_project(root: &Path) {
    let crud = root.join(".crud");
    fs::create_dir_all(crud.join("templates")).unwrap();
    let body = r#"package {{package}};
// model {{model_pascal}} table {{table}}
{{#each fields}}{{name}}:{{type}}
{{/each}}
List<String>
"#;
    fs::write(crud.join("templates/Entity.java.hbs"), body).unwrap();
    let cfg = SetupConfig::from_selections(SetupSelections {
        backend: Backend::None,
        frontend: Frontend::None,
        template: None,
    });
    fs::write(crud.join("setup.toml"), cfg.to_toml_pretty().unwrap()).unwrap();
}

fn collect_output_files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    fn walk(dir: &Path, prefix: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        let entries = fs::read_dir(dir).unwrap_or_else(|_| panic!("read_dir {}", dir.display()));
        for entry in entries {
            let entry = entry.expect("entry");
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == ".crud") {
                continue;
            }
            let rel = path.strip_prefix(prefix).expect("strip prefix");
            if path.is_dir() {
                walk(&path, prefix, out);
            } else if path.is_file() {
                let key = rel.to_string_lossy().replace('\\', "/");
                if key == "user.json" {
                    continue;
                }
                let bytes = fs::read(&path).expect("read output");
                out.insert(key, bytes);
            }
        }
    }
    walk(root, root, &mut out);
    out
}

#[test]
fn gen_file_and_fields_outputs_byte_identical() {
    let _eg = env_guard();
    std::env::remove_var("CRUD_AGENT");

    let dir_fields = tempfile::TempDir::new().unwrap();
    let dir_file = tempfile::TempDir::new().unwrap();
    seed_gen_project(dir_fields.path());
    seed_gen_project(dir_file.path());

    fs::write(
        dir_file.path().join("user.json"),
        r#"{
  "name": "User",
  "table": "sys_user",
  "package": "com.acme.demo",
  "fields": [
    { "name": "id", "type": "Long" },
    { "name": "name", "type": "String" }
  ]
}"#,
    )
    .unwrap();

    let fields_out = Command::new(exe())
        .current_dir(dir_fields.path())
        .args([
            "gen",
            "User",
            "--fields",
            "id:Long,name:String",
            "--package",
            "com.acme.demo",
            "--table",
            "sys_user",
        ])
        .output()
        .expect("gen --fields");
    assert!(
        fields_out.status.success(),
        "gen --fields failed: {}",
        String::from_utf8_lossy(&fields_out.stderr)
    );

    let file_out = Command::new(exe())
        .current_dir(dir_file.path())
        .args(["gen", "--file", "user.json"])
        .output()
        .expect("gen --file");
    assert!(
        file_out.status.success(),
        "gen --file failed: {}",
        String::from_utf8_lossy(&file_out.stderr)
    );

    let outputs_fields = collect_output_files(dir_fields.path());
    let outputs_file = collect_output_files(dir_file.path());

    assert_eq!(
        outputs_fields.keys().collect::<Vec<_>>(),
        outputs_file.keys().collect::<Vec<_>>(),
        "output path sets differ"
    );

    for (rel, bytes_fields) in &outputs_fields {
        let bytes_file = outputs_file
            .get(rel)
            .unwrap_or_else(|| panic!("missing output file in --file run: {rel}"));
        assert_eq!(
            bytes_fields,
            bytes_file,
            "byte mismatch for {rel}\n--fields:\n{}\n--file:\n{}",
            String::from_utf8_lossy(bytes_fields),
            String::from_utf8_lossy(bytes_file)
        );
    }
    assert!(
        !outputs_fields.is_empty(),
        "expected at least one output file"
    );
}
