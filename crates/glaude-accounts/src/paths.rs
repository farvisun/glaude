//! Account homes for spawning the tools. Mirrors account_manager.py.

use std::path::PathBuf;

use crate::tool::{Ctx, Kind};

/// `$MAMADOO_DIR` or `~/.claude_multi`.
pub fn root() -> PathBuf {
    Ctx::from_env().root
}

/// `CLAUDE_CONFIG_DIR` for a Claude account; `None` for `default`, which
/// must run without the variable (its keychain item has no path hash).
pub fn claude_config_dir(name: &str) -> Option<PathBuf> {
    (name != "default" && !name.is_empty()).then(|| root().join(name))
}

/// The folder holding a Claude account's `projects/`, settings, etc.
pub fn claude_home(name: &str) -> PathBuf {
    let ctx = Ctx::from_env();
    Kind::Claude.account_dir(&ctx, if name.is_empty() { "default" } else { name })
}

/// `CODEX_HOME` for a Codex account (always set, also for `default`).
pub fn codex_home(name: &str) -> PathBuf {
    let ctx = Ctx::from_env();
    Kind::Codex.account_dir(&ctx, if name.is_empty() { "default" } else { name })
}
