//! The two tools as mamadoo sees them: where homes live, what is shared,
//! and the small config files (archived names, active account, UI prefs).

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Result};
use serde_json::{json, Map, Value};

use crate::util::{self, expand_user, read_obj, write_json};

/// Paths resolved once from the environment; tests point them elsewhere.
#[derive(Debug, Clone)]
pub struct Ctx {
    pub home: PathBuf,
    /// `$MAMADOO_DIR` or `~/.claude_multi`.
    pub root: PathBuf,
    /// `$MAMADOO_CODEX_CONFIG` or `$ROOT/.mamadoo-codex.json`.
    pub codex_config: PathBuf,
    pub backup_root: PathBuf,
}

impl Ctx {
    pub fn from_env() -> Self {
        let env = glaude_core::env::login_env();
        let home = glaude_core::env::home_dir();
        let get = |k: &str| env.get(k).filter(|v| !v.is_empty()).cloned();
        Self::build(home, get("MAMADOO_DIR"), get("MAMADOO_CODEX_CONFIG"))
    }

    /// A context rooted at another HOME (parity tests, scratch copies).
    pub fn with_home(home: &Path) -> Self {
        Self::build(home.to_path_buf(), None, None)
    }

    fn build(home: PathBuf, root: Option<String>, codex_config: Option<String>) -> Self {
        let root = root
            .map(|r| expand_user(&r, &home))
            .unwrap_or_else(|| home.join(".claude_multi"));
        let codex_config = codex_config
            .map(|c| expand_user(&c, &home))
            .unwrap_or_else(|| root.join(".mamadoo-codex.json"));
        Self {
            backup_root: home.join(".agent-accounts-backup"),
            home,
            root,
            codex_config,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Claude,
    Codex,
}

pub const CLAUDE_SHARED: &[&str] = &[
    "projects",
    "history.jsonl",
    "file-history",
    "paste-cache",
    "plans",
    "todos",
    "tasks",
    "jobs",
    "sessions",
    "session-env",
    "shell-snapshots",
    "ide",
    "settings.json",
    "CLAUDE.md",
    "RTK.md",
    "keybindings.json",
    "agents",
    "skills",
    "commands",
    "hooks",
    "output-styles",
    "plugins",
    "statusline",
    "statusline-custom.sh",
    "context-mode",
    ".caveman-active",
    ".caveman-mode-log.jsonl",
];
pub const CLAUDE_DATA_DIRS: &[&str] = &[
    "projects",
    "file-history",
    "paste-cache",
    "plans",
    "todos",
    "tasks",
    "jobs",
    "sessions",
    "session-env",
    "shell-snapshots",
    "ide",
];
pub const CODEX_SHARED: &[&str] = &[
    "sessions",
    "archived_sessions",
    "history.jsonl",
    "session_index.jsonl",
    "state_5.sqlite",
    "thread_history_1.sqlite",
    "goals_1.sqlite",
    "memories_1.sqlite",
    "memories",
    "config.toml",
    "AGENTS.md",
    "RTK.md",
    "hooks.json",
    "skills",
    "plugins",
    "prompts",
    "rules",
    "shell_snapshots",
    "generated_images",
    "attachments",
    "visualizations",
];
pub const CODEX_DATA_DIRS: &[&str] = &[
    "sessions",
    "archived_sessions",
    "memories",
    "shell_snapshots",
    "generated_images",
    "attachments",
    "visualizations",
];

/// One account on disk.
#[derive(Debug, Clone)]
pub struct Account {
    pub kind: Kind,
    pub name: String,
    pub path: PathBuf,
    pub archived: bool,
    pub protected: bool,
    pub meta: Map<String, Value>,
    pub usage: Map<String, Value>,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Claude => "claude",
            Kind::Codex => "codex",
        }
    }

    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "claude" => Ok(Kind::Claude),
            "codex" => Ok(Kind::Codex),
            other => bail!("Unknown tool '{other}'"),
        }
    }

    pub fn central(self, ctx: &Ctx) -> PathBuf {
        match self {
            Kind::Claude => ctx.home.join(".claude"),
            Kind::Codex => ctx.home.join(".codex"),
        }
    }

    pub fn shared(self) -> &'static [&'static str] {
        match self {
            Kind::Claude => CLAUDE_SHARED,
            Kind::Codex => CODEX_SHARED,
        }
    }

    pub fn data_dirs(self) -> &'static [&'static str] {
        match self {
            Kind::Claude => CLAUDE_DATA_DIRS,
            Kind::Codex => CODEX_DATA_DIRS,
        }
    }

    pub fn home_env(self) -> &'static str {
        match self {
            Kind::Claude => "CLAUDE_CONFIG_DIR",
            Kind::Codex => "CODEX_HOME",
        }
    }

    fn config_path(self, ctx: &Ctx) -> PathBuf {
        match self {
            Kind::Claude => ctx.root.join(".mamadoo.json"),
            Kind::Codex => ctx.codex_config.clone(),
        }
    }

    pub fn config(self, ctx: &Ctx) -> Map<String, Value> {
        read_obj(&self.config_path(ctx))
    }

    fn save_config(self, ctx: &Ctx, data: Map<String, Value>) -> Result<()> {
        write_json(&self.config_path(ctx), &Value::Object(data), true)
    }

    pub fn archived_names(self, ctx: &Ctx) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(a) = self.config(ctx).get("archived").and_then(Value::as_array) {
            for v in a.iter().filter_map(Value::as_str) {
                if !out.iter().any(|x| x == v) {
                    out.push(v.to_string());
                }
            }
        }
        out
    }

    pub fn set_archived(self, ctx: &Ctx, name: &str, archived: bool) -> Result<()> {
        let mut data = self.config(ctx);
        let mut names: Vec<String> = self
            .archived_names(ctx)
            .into_iter()
            .filter(|a| a != name)
            .collect();
        if archived {
            names.push(name.to_string());
        }
        data.insert("archived".into(), json!(names));
        self.save_config(ctx, data)
    }

    pub fn get_pref(self, ctx: &Ctx, key: &str, default: &str) -> String {
        self.config(ctx)
            .get("ui")
            .and_then(|u| u.get(key))
            .map(|v| util::py_str(Some(v)))
            .unwrap_or_else(|| default.to_string())
    }

    pub fn set_pref(self, ctx: &Ctx, key: &str, value: &str) -> Result<()> {
        let mut data = self.config(ctx);
        let ui = data
            .entry("ui")
            .or_insert_with(|| Value::Object(Map::new()));
        if !ui.is_object() {
            *ui = Value::Object(Map::new());
        }
        ui[key] = json!(value);
        self.save_config(ctx, data)
    }

    pub fn active(self, ctx: &Ctx) -> String {
        match self {
            Kind::Claude => fs::read_to_string(ctx.root.join(".active"))
                .map(|s| s.trim().to_string())
                .unwrap_or_default(),
            Kind::Codex => self
                .config(ctx)
                .get("active")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
        }
    }

    pub fn set_active(self, ctx: &Ctx, name: Option<&str>) -> Result<()> {
        match self {
            Kind::Claude => {
                let path = ctx.root.join(".active");
                match name.filter(|n| !n.is_empty()) {
                    Some(n) => {
                        fs::create_dir_all(&ctx.root)?;
                        fs::write(path, format!("{n}\n"))?;
                    }
                    None => {
                        let _ = fs::remove_file(path);
                    }
                }
                Ok(())
            }
            Kind::Codex => {
                let mut data = self.config(ctx);
                match name.filter(|n| !n.is_empty()) {
                    Some(n) => {
                        data.insert("active".into(), json!(n));
                    }
                    None => {
                        data.remove("active");
                    }
                }
                self.save_config(ctx, data)
            }
        }
    }

    pub fn account_dir(self, ctx: &Ctx, name: &str) -> PathBuf {
        match self {
            Kind::Claude if name == "default" => self.central(ctx),
            Kind::Claude => ctx.root.join(name),
            Kind::Codex if name == "default" => ctx.home.join(".codex"),
            Kind::Codex => ctx.home.join(format!(".codex_{name}")),
        }
    }

    /// `(name, path, protected)`; default first, the rest by lowercase name.
    pub fn discover(self, ctx: &Ctx) -> Vec<(String, PathBuf, bool)> {
        let mut out = Vec::new();
        let central = self.central(ctx);
        if central.is_dir() {
            out.push(("default".to_string(), central, true));
        }
        let mut rest: Vec<(String, PathBuf, bool)> = match self {
            Kind::Claude => fs::read_dir(&ctx.root)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let name = e.file_name().to_str()?.to_string();
                    let p = e.path();
                    (p.is_dir() && !name.starts_with('.')).then_some((name, p, false))
                })
                .collect(),
            Kind::Codex => fs::read_dir(&ctx.home)
                .into_iter()
                .flatten()
                .flatten()
                .filter_map(|e| {
                    let file = e.file_name().to_str()?.to_string();
                    let short = file.strip_prefix(".codex_")?.to_string();
                    let p = e.path();
                    (p.is_dir() && !short.is_empty()).then_some((short, p, false))
                })
                .collect(),
        };
        rest.sort_by_key(|(_, p, _)| {
            p.file_name()
                .map(|n| n.to_string_lossy().to_lowercase())
                .unwrap_or_default()
        });
        out.extend(rest);
        out
    }

    /// Where mamadoo keeps `.meta.json` / `.usage.json` for an account.
    pub fn state_dir(self, ctx: &Ctx, acct: &Account) -> PathBuf {
        match self {
            Kind::Claude if self.is_default(ctx, acct) => ctx.root.join(".default"),
            _ => acct.path.clone(),
        }
    }

    pub fn is_default(self, ctx: &Ctx, acct: &Account) -> bool {
        acct.path == self.central(ctx)
    }

    pub fn accounts(self, ctx: &Ctx, include_archived: bool) -> Vec<Account> {
        let archived = self.archived_names(ctx);
        self.discover(ctx)
            .into_iter()
            .filter_map(|(name, path, protected)| {
                let is_archived = archived.contains(&name);
                if is_archived && !include_archived {
                    return None;
                }
                let mut acct = Account {
                    kind: self,
                    name,
                    path,
                    archived: is_archived,
                    protected,
                    meta: Map::new(),
                    usage: Map::new(),
                };
                let state = self.state_dir(ctx, &acct);
                acct.meta = read_obj(&state.join(".meta.json"));
                acct.usage = read_obj(&state.join(".usage.json"));
                Some(acct)
            })
            .collect()
    }

    pub fn find(self, ctx: &Ctx, name: &str) -> Result<Account> {
        self.accounts(ctx, true)
            .into_iter()
            .find(|a| a.name == name)
            .ok_or_else(|| anyhow::anyhow!("Account '{name}' not found."))
    }

    pub fn check_new_name(self, ctx: &Ctx, raw: &str) -> Result<String> {
        let name = util::normalize_name(raw);
        if name == "default" {
            match self {
                Kind::Claude => bail!("'default' is reserved for ~/.claude."),
                Kind::Codex => bail!("'default' is reserved for ~/.codex."),
            }
        }
        util::check_name_shape(&name)?;
        let dir = self.account_dir(ctx, &name);
        if dir.exists() {
            bail!(
                "Account '{name}' already exists at {}",
                util::tilde(&dir, &ctx.home)
            );
        }
        Ok(name)
    }

    pub fn create(self, ctx: &Ctx, raw: &str) -> Result<Account> {
        let name = self.check_new_name(ctx, raw)?;
        let path = self.account_dir(ctx, &name);
        fs::create_dir_all(&path)?;
        write_json(
            &path.join(".meta.json"),
            &json!({"created_at": util::iso(Some(util::now_utc())), "last_used": "", "session_count": 0}),
            true,
        )?;
        self.set_archived(ctx, &name, false)?;
        self.find(ctx, &name)
    }

    pub fn rename(self, ctx: &Ctx, old: &str, raw_new: &str) -> Result<String> {
        let acct = self.find(ctx, old)?;
        if acct.protected {
            bail!("Cannot rename the protected '{old}' account.");
        }
        if util::normalize_name(raw_new) == old {
            bail!("New name is the same as the old name.");
        }
        let new = self.check_new_name(ctx, raw_new)?;
        if self == Kind::Codex && is_symlink(&acct.path.join("state_5.sqlite")) {
            crate::share::codex_repoint(ctx, &acct)?;
        }
        fs::rename(&acct.path, self.account_dir(ctx, &new))?;
        if acct.archived {
            self.set_archived(ctx, old, false)?;
            self.set_archived(ctx, &new, true)?;
        }
        if self.active(ctx) == old {
            self.set_active(ctx, Some(&new))?;
        }
        Ok(new)
    }

    pub fn delete(self, ctx: &Ctx, name: &str) -> Result<()> {
        let acct = self.find(ctx, name)?;
        if acct.protected {
            bail!(
                "Refusing to delete the protected '{name}' account ({}).",
                util::tilde(&acct.path, &ctx.home)
            );
        }
        if self == Kind::Codex && is_symlink(&acct.path.join("state_5.sqlite")) {
            crate::share::codex_repoint(ctx, &acct)?;
        }
        fs::remove_dir_all(&acct.path)?;
        self.set_archived(ctx, name, false)?;
        if self.active(ctx) == name {
            self.set_active(ctx, None)?;
        }
        Ok(())
    }

    /// Record a launch: `last_used`, `session_count + 1`.
    pub fn bump(self, ctx: &Ctx, acct: &Account) -> Result<()> {
        let mut meta = acct.meta.clone();
        let now = util::iso(Some(util::now_utc()));
        meta.entry("created_at").or_insert_with(|| json!(now));
        meta.insert("last_used".into(), json!(now));
        let n = meta.get("session_count").map(util::as_u64).unwrap_or(0) + 1;
        meta.insert("session_count".into(), json!(n));
        write_json(
            &self.state_dir(ctx, acct).join(".meta.json"),
            &Value::Object(meta),
            true,
        )
    }
}

pub fn is_symlink(p: &Path) -> bool {
    fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}
