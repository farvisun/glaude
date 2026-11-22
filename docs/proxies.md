# Proxies

Settings → Network keeps a list of proxies. Each one has a name, a URL
(`http://`, `https://` or `socks5://`, with an optional `user:password@`)
and a list of hosts that skip it (`NO_PROXY`; empty means
`localhost,127.0.0.1,::1`).

## Where the list lives

`proxies.json` in the app data folder (`~/Library/Application Support/com.farvisun.glaude`
on macOS), written with mode 0600. Not in the webview's storage, because a
URL can carry a password. The agent host reads the same file when it starts
a process, so a conversation only stores the proxy's id.

## Which proxy a process uses

Most specific first:

1. The conversation's own choice (a proxy, or *No proxy*), from the New
   conversation dialog or the statusline.
2. The project default: the closest folder with a rule, so a rule on a
   project also covers the folders (and worktrees) inside it.
3. The account default (`claude:<name>` or `codex:<name>`).
4. The app default.

Nothing set: the process gets the environment of the login shell as before.

Terminals in the panel use the focused conversation's proxy, or the project
or app default when no conversation is focused, unless *Terminals use the
proxy* is off. Terminals that run an agent (*Open in terminal*, account
login) always follow that conversation or account.

## How a process gets it

The variables `HTTPS_PROXY`, `HTTP_PROXY`, `ALL_PROXY` and `NO_PROXY`, in
upper and lower case, are added to its environment.

- Claude Code honours `HTTPS_PROXY`/`HTTP_PROXY`/`NO_PROXY` and supports
  basic auth in the URL; it does not support SOCKS (Anthropic's network
  configuration docs). The UI warns when a SOCKS proxy is picked for Claude.
- Codex runs one `app-server` per account and proxy, so threads that go out
  different ways never share a process.
- Changing a running conversation's proxy stops its process and resumes the
  same session with the new environment. Not allowed while a turn runs.

Checked on 2026-11-22 with a local CONNECT proxy: `claude -p` and
`codex exec` sent all their traffic (API, MCP servers) through it.

## Check

*Check* runs `curl` through the proxy (the URL goes in the environment, not
the arguments, so the password never shows in the process list) against
`api.anthropic.com`, `api.openai.com` and Cloudflare's trace page. Any HTTP
answer counts as reachable; the trace page gives the exit address and
country.
