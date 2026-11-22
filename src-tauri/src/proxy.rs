//! Saved proxies (Settings → Network). Kept in a private file in the app's
//! data folder, not in the webview's storage, because URLs can carry a
//! password. Conversations and terminals refer to a proxy by id; the agent
//! host reads the file when it starts a process.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use glaude_core::proxy::ProxyEnv;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Proxy {
    pub id: String,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub no_proxy: String,
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxySettings {
    #[serde(default)]
    pub proxies: Vec<Proxy>,
    /// Used when nothing more specific is set.
    #[serde(default)]
    pub app_default: Option<String>,
    /// "claude:work" → proxy id, or "none".
    #[serde(default)]
    pub by_account: BTreeMap<String, String>,
    /// Project folder → proxy id, or "none".
    #[serde(default)]
    pub by_project: BTreeMap<String, String>,
    /// Terminals opened in Glaude use the proxy too.
    #[serde(default = "yes")]
    pub terminals: bool,
}

impl Default for ProxySettings {
    fn default() -> Self {
        Self {
            proxies: Vec::new(),
            app_default: None,
            by_account: BTreeMap::new(),
            by_project: BTreeMap::new(),
            terminals: true,
        }
    }
}

fn path() -> PathBuf {
    crate::host::data_dir().join("proxies.json")
}

pub fn load() -> ProxySettings {
    std::fs::read(path())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn save(s: &ProxySettings) -> Result<(), String> {
    let p = path();
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = p.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(
        &tmp,
        serde_json::to_vec_pretty(s).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// The proxy a process should use: `id` is a saved proxy's id; missing,
/// empty or "none" means no proxy (the login shell's variables stay).
pub fn resolve(id: Option<&str>) -> Option<ProxyEnv> {
    let id = id.filter(|i| !i.is_empty() && *i != "none")?;
    load()
        .proxies
        .into_iter()
        .find(|p| p.id == id)
        .filter(|p| !p.url.trim().is_empty())
        .map(|p| ProxyEnv {
            url: p.url,
            no_proxy: p.no_proxy,
        })
}

/// The proxy for app-side calls of an account (model lists, thread lists):
/// the account's default, else the app default.
pub fn for_account(provider: &str, account: &str) -> Option<ProxyEnv> {
    let s = load();
    let id = s
        .by_account
        .get(&format!("{provider}:{account}"))
        .cloned()
        .or(s.app_default);
    resolve(id.as_deref())
}

#[tauri::command]
pub fn proxies_get() -> ProxySettings {
    load()
}

#[tauri::command]
pub fn proxies_set(settings: ProxySettings) -> Result<(), String> {
    for p in &settings.proxies {
        let u = p.url.trim().to_ascii_lowercase();
        if ![
            "http://",
            "https://",
            "socks5://",
            "socks5h://",
            "socks4://",
        ]
        .iter()
        .any(|s| u.starts_with(s))
        {
            return Err(format!(
                "{}: the address must start with http://, https:// or socks5://",
                p.name
            ));
        }
    }
    save(&settings)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Probe {
    pub target: String,
    /// Any HTTP answer means the proxy got through.
    pub ok: bool,
    pub status: Option<u16>,
    pub ms: Option<u64>,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckResult {
    pub probes: Vec<Probe>,
    /// Exit address and country seen by Cloudflare.
    pub ip: Option<String>,
    pub country: Option<String>,
}

/// One request through the proxy with curl. The proxy goes in the
/// environment, so a password never shows in the process list.
fn probe(proxy: &str, label: &str, url: &str) -> (Probe, String) {
    let start = Instant::now();
    let out = Command::new("curl")
        .args([
            "-sS",
            "--max-time",
            "12",
            "-o",
            "-",
            "-w",
            "\n%{http_code}",
            url,
        ])
        .env_remove("NO_PROXY")
        .env_remove("no_proxy")
        .env("HTTPS_PROXY", proxy)
        .env("https_proxy", proxy)
        .env("ALL_PROXY", proxy)
        .output();
    let ms = start.elapsed().as_millis() as u64;
    match out {
        Ok(o) => {
            let text = String::from_utf8_lossy(&o.stdout).to_string();
            let (body, code) = text.rsplit_once('\n').unwrap_or(("", text.as_str()));
            let status = code.trim().parse::<u16>().ok().filter(|c| *c > 0);
            let err = String::from_utf8_lossy(&o.stderr).trim().to_string();
            (
                Probe {
                    target: label.into(),
                    ok: status.is_some(),
                    status,
                    ms: status.map(|_| ms),
                    error: (status.is_none()).then(|| {
                        if err.is_empty() {
                            "No answer".into()
                        } else {
                            err.trim_start_matches("curl: ").to_string()
                        }
                    }),
                },
                body.to_string(),
            )
        }
        Err(e) => (
            Probe {
                target: label.into(),
                ok: false,
                status: None,
                ms: None,
                error: Some(format!("curl is not available: {e}")),
            },
            String::new(),
        ),
    }
}

/// Reach Anthropic and OpenAI through a proxy, and see where it exits.
#[tauri::command]
pub async fn proxy_check(url: String) -> Result<CheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let targets = [
            ("Anthropic", "https://api.anthropic.com/v1/models"),
            ("OpenAI", "https://api.openai.com/v1/models"),
            ("Exit address", "https://www.cloudflare.com/cdn-cgi/trace"),
        ];
        let handles: Vec<_> = targets
            .iter()
            .map(|(label, target)| {
                let (u, l, t) = (url.clone(), label.to_string(), target.to_string());
                std::thread::spawn(move || probe(&u, &l, &t))
            })
            .collect();
        let mut probes = Vec::new();
        let (mut ip, mut country) = (None, None);
        for h in handles {
            let (p, body) = h.join().map_err(|_| "The check failed".to_string())?;
            if p.target == "Exit address" && p.ok {
                for line in body.lines() {
                    if let Some(v) = line.strip_prefix("ip=") {
                        ip = Some(v.to_string());
                    } else if let Some(v) = line.strip_prefix("loc=") {
                        country = Some(v.to_string());
                    }
                }
            }
            probes.push(p);
        }
        Ok(CheckResult {
            probes,
            ip,
            country,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unreachable_proxy_fails_every_probe() {
        let r = tauri::async_runtime::block_on(proxy_check("http://127.0.0.1:9".into())).unwrap();
        assert_eq!(r.probes.len(), 3);
        assert!(r.probes.iter().all(|p| !p.ok && p.error.is_some()));
        assert!(r.ip.is_none());
    }

    /// `GLAUDE_TEST_PROXY=http://127.0.0.1:8899 cargo test -p glaude -- --ignored proxy`
    #[test]
    #[ignore = "needs GLAUDE_TEST_PROXY and network access"]
    fn real_proxy_reaches_both() {
        let url = std::env::var("GLAUDE_TEST_PROXY").expect("GLAUDE_TEST_PROXY");
        let r = tauri::async_runtime::block_on(proxy_check(url)).unwrap();
        assert!(r.probes.iter().all(|p| p.ok), "{r:?}");
        assert!(r.ip.is_some());
    }
}
