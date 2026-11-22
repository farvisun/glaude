//! A proxy for agent processes and terminals, given to them through the
//! standard variables: HTTPS_PROXY, HTTP_PROXY, ALL_PROXY and NO_PROXY, in
//! upper and lower case.

use serde::{Deserialize, Serialize};

/// Hosts that never go through the proxy when none are given (local MCP
/// servers, dev servers).
pub const DEFAULT_NO_PROXY: &str = "localhost,127.0.0.1,::1";

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyEnv {
    /// `http://user:pass@host:port`, `https://…` or `socks5://…`.
    pub url: String,
    /// Comma-separated hosts to reach directly.
    #[serde(default)]
    pub no_proxy: String,
}

impl ProxyEnv {
    /// The variables to set (overrides for `env::agent_env`).
    pub fn vars(&self) -> Vec<(&'static str, Option<String>)> {
        let url = Some(self.url.trim().to_string());
        let no = self.no_proxy.trim();
        let no = Some(if no.is_empty() { DEFAULT_NO_PROXY } else { no }.to_string());
        vec![
            ("HTTPS_PROXY", url.clone()),
            ("https_proxy", url.clone()),
            ("HTTP_PROXY", url.clone()),
            ("http_proxy", url.clone()),
            ("ALL_PROXY", url.clone()),
            ("all_proxy", url),
            ("NO_PROXY", no.clone()),
            ("no_proxy", no),
        ]
    }

    /// SOCKS proxies work for Codex and terminals, not for Claude Code.
    pub fn is_socks(&self) -> bool {
        self.url
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("socks")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_every_variable_and_a_default_bypass() {
        let p = ProxyEnv {
            url: " http://u:p@proxy:8080 ".into(),
            no_proxy: String::new(),
        };
        let vars = p.vars();
        assert!(vars.contains(&("HTTPS_PROXY", Some("http://u:p@proxy:8080".into()))));
        assert!(vars.contains(&("all_proxy", Some("http://u:p@proxy:8080".into()))));
        assert!(vars.contains(&("NO_PROXY", Some(DEFAULT_NO_PROXY.into()))));
        assert!(!p.is_socks());
        assert!(ProxyEnv {
            url: "SOCKS5://h:1".into(),
            no_proxy: "x".into()
        }
        .is_socks());
    }
}
