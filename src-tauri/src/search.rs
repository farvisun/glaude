//! History search commands. The index lives in the app data folder and is
//! brought up to date in the background: at startup, when the palette
//! opens, and after turns end.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use glaude_accounts::{tool::is_symlink, Ctx, Kind as Tool};
use glaude_search::{Hit, Index, Kind, Source};
use parking_lot::Mutex;
use serde::Serialize;
use tauri::{AppHandle, Manager, State};

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub ready: bool,
    pub indexing: bool,
    pub progress: f32,
    pub messages: u64,
    pub error: Option<String>,
}

#[derive(Default)]
pub struct Search {
    index: Mutex<Option<Arc<Index>>>,
    status: Mutex<Status>,
    running: AtomicBool,
}

/// Session folders of every account that keeps its own copy (linked
/// accounts point at the central home and are covered by it).
fn sources() -> Vec<Source> {
    let ctx = Ctx::from_env();
    let mut out = Vec::new();
    let mut add = |kind: Kind, root: PathBuf| {
        if root.is_dir() && !is_symlink(&root) && !out.iter().any(|s: &Source| s.root == root) {
            out.push(Source { kind, root });
        }
    };
    for acct in Tool::Claude.accounts(&ctx, true) {
        add(Kind::Claude, acct.path.join("projects"));
    }
    for acct in Tool::Codex.accounts(&ctx, true) {
        add(Kind::Codex, acct.path.join("sessions"));
        add(Kind::Codex, acct.path.join("archived_sessions"));
    }
    out
}

impl Search {
    fn index(&self, app: &AppHandle) -> anyhow::Result<Arc<Index>> {
        let mut slot = self.index.lock();
        if let Some(i) = slot.as_ref() {
            return Ok(i.clone());
        }
        let dir = app.path().app_data_dir()?;
        let idx = Arc::new(Index::open(&dir.join("search.sqlite"))?);
        *slot = Some(idx.clone());
        Ok(idx)
    }

    /// Start an incremental update unless one is running.
    pub fn refresh(self: &Arc<Self>, app: AppHandle) {
        if self.running.swap(true, Ordering::SeqCst) {
            return;
        }
        let me = self.clone();
        std::thread::Builder::new()
            .name("search-index".into())
            .spawn(move || {
                {
                    let mut s = me.status.lock();
                    s.indexing = true;
                    s.progress = 0.0;
                }
                let res = me.index(&app).and_then(|idx| {
                    let st = idx.update(&sources(), &mut |p| me.status.lock().progress = p)?;
                    Ok((st, idx.message_count()))
                });
                let mut s = me.status.lock();
                s.indexing = false;
                match res {
                    Ok((st, total)) => {
                        tracing::info!(
                            "search index: {} files read, {} new messages",
                            st.scanned,
                            st.messages
                        );
                        s.ready = true;
                        s.messages = total;
                        s.progress = 1.0;
                        s.error = None;
                    }
                    Err(e) => {
                        tracing::warn!("search index: {e:#}");
                        s.error = Some(format!("{e:#}"));
                    }
                }
                drop(s);
                me.running.store(false, Ordering::SeqCst);
            })
            .ok();
    }
}

/// Managed state wrapper so commands get the `Arc`.
#[derive(Default, Clone)]
pub struct SearchState(pub Arc<Search>);

#[tauri::command]
pub fn search_status(state: State<'_, SearchState>) -> Status {
    state.0.status.lock().clone()
}

#[tauri::command]
pub fn search_refresh(app: AppHandle, state: State<'_, SearchState>) {
    state.0.refresh(app);
}

#[tauri::command]
pub async fn search_query(
    app: AppHandle,
    state: State<'_, SearchState>,
    query: String,
    limit: Option<usize>,
) -> Result<Vec<Hit>, String> {
    let search = state.0.clone();
    tokio::task::spawn_blocking(move || {
        let idx = search.index(&app)?;
        idx.search(&query, limit.unwrap_or(50).min(200))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}
