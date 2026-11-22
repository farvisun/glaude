//! Where mamadoo keeps account homes. Mirrors account_manager.py.

use std::path::PathBuf;

use glaude_core::env;

/// `$MAMADOO_DIR` or `~/.claude_multi`.
pub fn root() -> PathBuf {
    env::login_env()
        .get("MAMADOO_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| env::home_dir().join(".claude_multi"))
}

/// `CLAUDE_CONFIG_DIR` for a Claude account; `None` for `default`, which
/// must run without the variable (its keychain item has no path hash).
pub fn claude_config_dir(name: &str) -> Option<PathBuf> {
    (name != "default" && !name.is_empty()).then(|| root().join(name))
}

/// The folder holding a Claude account's `projects/`, settings, etc.
pub fn claude_home(name: &str) -> PathBuf {
    claude_config_dir(name).unwrap_or_else(|| env::home_dir().join(".claude"))
}

/// `CODEX_HOME` for a Codex account (always set, also for `default`).
pub fn codex_home(name: &str) -> PathBuf {
    if name == "default" || name.is_empty() {
        env::home_dir().join(".codex")
    } else {
        env::home_dir().join(format!(".codex_{name}"))
    }
}
