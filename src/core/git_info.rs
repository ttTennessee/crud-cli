//! Git user identity for template context (D-G33).

/// `git config user.name` / `user.email` (empty when unavailable).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitInfo {
    pub user_name: String,
    pub user_email: String,
}

/**
 * Reads `user.name` and `user.email` via `git config --get` (argv, no shell).
 *
 * Never returns `Err`; missing git or unset config yields empty strings.
 */
pub fn read() -> GitInfo {
    GitInfo {
        user_name: git_config_value("user.name"),
        user_email: git_config_value("user.email"),
    }
}

fn git_config_value(key: &str) -> String {
    let output = std::process::Command::new("git")
        .args(["config", "--get", key])
        .output();

    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .trim()
            .to_string(),
        _ => String::new(),
    }
}
