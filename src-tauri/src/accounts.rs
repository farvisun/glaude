//! Accounts page commands (mamadoo port). File work runs on the blocking
//! pool; scans of one tool run one at a time because linked accounts share
//! a scan index.

use std::sync::Mutex;

use glaude_accounts::{share, AccountView, Ctx, Kind, Limit};
use glaude_core::model::LimitWindow;
use serde::Serialize;
use tauri::ipc::Channel;
use tauri::State;

use crate::conv::CodexPool;

static CLAUDE_SCAN: Mutex<()> = Mutex::new(());
static CODEX_SCAN: Mutex<()> = Mutex::new(());

fn err(e: impl std::fmt::Display) -> String {
    e.to_string()
}

fn kind(provider: &str) -> Result<Kind, String> {
    Kind::parse(provider).map_err(err)
}

async fn blocking<T: Send + 'static>(
    f: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> Result<T, String> {
    tokio::task::spawn_blocking(f)
        .await
        .map_err(err)?
        .map_err(|e| format!("{e:#}"))
}

fn to_limit(w: &LimitWindow) -> Limit {
    Limit {
        used: w.used,
        resets_at: w.resets_at.clone(),
    }
}

/// Live Codex limits from an app-server Glaude already runs for that home;
/// never starts one just for this.
async fn codex_live(pool: &CodexPool, view: &mut AccountView) {
    let ctx = Ctx::from_env();
    let home = Kind::Codex.account_dir(&ctx, &view.name);
    let Some(server) = pool.0.existing(&home).await else {
        return;
    };
    if let Ok((_, limits)) = server.account().await {
        if limits.five_hour.is_some() || limits.week.is_some() {
            view.profile.five_hour = limits.five_hour.as_ref().map(to_limit);
            view.profile.week = limits.week.as_ref().map(to_limit);
            view.profile.limits_as_of =
                glaude_accounts::util::iso(Some(glaude_accounts::util::now_utc()));
            if let Some(plan) = limits.plan.filter(|p| !p.is_empty()) {
                view.profile.plan = plan;
            }
        }
    }
}

#[tauri::command]
pub async fn accounts_list(
    pool: State<'_, CodexPool>,
    provider: Option<String>,
) -> Result<Vec<AccountView>, String> {
    let kinds: Vec<Kind> = match provider.as_deref() {
        Some(p) => vec![kind(p)?],
        None => vec![Kind::Claude, Kind::Codex],
    };
    let mut out = blocking(move || {
        let ctx = Ctx::from_env();
        Ok(kinds
            .into_iter()
            .flat_map(|k| glaude_accounts::list(&ctx, k))
            .collect::<Vec<_>>())
    })
    .await?;
    for v in out.iter_mut().filter(|v| v.provider == Kind::Codex) {
        codex_live(&pool, v).await;
    }
    Ok(out)
}

#[tauri::command]
pub async fn accounts_refresh(
    pool: State<'_, CodexPool>,
    provider: String,
    name: String,
    on_progress: Channel<f32>,
) -> Result<AccountView, String> {
    let k = kind(&provider)?;
    let mut view = blocking(move || {
        let lock = match k {
            Kind::Claude => &CLAUDE_SCAN,
            Kind::Codex => &CODEX_SCAN,
        };
        let _guard = lock.lock().unwrap_or_else(|p| p.into_inner());
        let ctx = Ctx::from_env();
        let mut last = 0.0f32;
        glaude_accounts::refresh(&ctx, k, &name, &mut |f| {
            if f - last >= 0.02 || f >= 1.0 {
                last = f;
                let _ = on_progress.send(f);
            }
        })
    })
    .await?;
    if k == Kind::Codex {
        codex_live(&pool, &mut view).await;
    }
    Ok(view)
}

#[tauri::command]
pub async fn accounts_create(provider: String, name: String) -> Result<String, String> {
    let k = kind(&provider)?;
    blocking(move || Ok(k.create(&Ctx::from_env(), &name)?.name)).await
}

#[tauri::command]
pub async fn accounts_rename(
    provider: String,
    name: String,
    new_name: String,
) -> Result<String, String> {
    let k = kind(&provider)?;
    blocking(move || k.rename(&Ctx::from_env(), &name, &new_name)).await
}

#[tauri::command]
pub async fn accounts_delete(provider: String, name: String) -> Result<(), String> {
    let k = kind(&provider)?;
    blocking(move || {
        let ctx = Ctx::from_env();
        let acct = k.find(&ctx, &name)?;
        if share::is_running(k, &acct) {
            anyhow::bail!("'{name}' is running. Close its sessions first.");
        }
        k.delete(&ctx, &name)
    })
    .await
}

#[tauri::command]
pub async fn accounts_archive(
    provider: String,
    name: String,
    archived: bool,
) -> Result<(), String> {
    let k = kind(&provider)?;
    blocking(move || k.set_archived(&Ctx::from_env(), &name, archived)).await
}

#[tauri::command]
pub async fn accounts_set_active(provider: String, name: String) -> Result<(), String> {
    let k = kind(&provider)?;
    blocking(move || k.set_active(&Ctx::from_env(), Some(&name))).await
}

#[tauri::command]
pub async fn accounts_share(
    provider: String,
    name: String,
    dry_run: bool,
) -> Result<Vec<String>, String> {
    let k = kind(&provider)?;
    blocking(move || {
        let ctx = Ctx::from_env();
        let mut acct = k.find(&ctx, &name)?;
        share::share(&ctx, k, &mut acct, dry_run)
    })
    .await
}

#[tauri::command]
pub async fn accounts_pref(
    provider: String,
    key: String,
    value: Option<String>,
) -> Result<String, String> {
    let k = kind(&provider)?;
    blocking(move || {
        let ctx = Ctx::from_env();
        if let Some(v) = &value {
            k.set_pref(&ctx, &key, v)?;
        }
        Ok(k.get_pref(&ctx, &key, ""))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountBrief {
    pub provider: Kind,
    pub name: String,
    pub email: Option<String>,
    pub active: bool,
    pub archived: bool,
}

/// Small list for pickers (new conversation dialog).
#[tauri::command]
pub async fn accounts_brief() -> Result<Vec<AccountBrief>, String> {
    blocking(|| {
        let ctx = Ctx::from_env();
        let mut out = Vec::new();
        for k in [Kind::Claude, Kind::Codex] {
            let active = k.active(&ctx);
            for a in k.accounts(&ctx, true) {
                let email = glaude_accounts::read_profile(&ctx, &a).email;
                out.push(AccountBrief {
                    provider: k,
                    active: a.name == active,
                    archived: a.archived,
                    email: (!email.is_empty()).then_some(email),
                    name: a.name,
                });
            }
        }
        Ok(out)
    })
    .await
}

/// mamadoo's launch steps before a conversation starts: make the home,
/// share (never blocks), count the launch, mark the account active.
pub async fn prepare_launch(provider: &str, account: &str, bump: bool) -> Vec<String> {
    let Ok(k) = Kind::parse(provider) else {
        return Vec::new();
    };
    let account = if account.is_empty() {
        "default"
    } else {
        account
    }
    .to_string();
    let res = blocking(move || share::prepare_launch(&Ctx::from_env(), k, &account, bump)).await;
    match res {
        Ok(log) => log,
        Err(e) => {
            tracing::warn!("launch bookkeeping: {e}");
            Vec::new()
        }
    }
}
