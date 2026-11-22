# Glaude

A fast desktop UI for **Claude Code** and **Codex**, built for Persian and
other right-to-left text. It drives the CLIs already installed and logged
in on your computer: no separate login and no API key.

Terminals have no bidi support, so Persian comes out reversed and mixed
Persian/English lines jump around. Glaude renders every message block with
its own direction, keeps code left-to-right, and uses the Estedad font.

## Features

- **Conversations** with Claude Code (stream-json) and Codex (`codex app-server`):
  streaming replies, thinking, tool calls, command output, plan/todo
  updates, interrupt and resume. Idle processes stop after 15 minutes and
  resume on the next message.
- **RTL done per block**: a block is right-to-left when most of its letters
  are; Latin runs inside Persian are isolated, code is always LTR. The
  composer switches direction as you type.
- **Permission cards** inside the conversation (allow, always allow, deny),
  with the diff for edits. Other tabs keep working while one waits.
- **Diff viewer** (unified or side by side) for Claude edits and Codex file changes.
- **Sidebar, tabs and splits**: up to four panes; drag a tab to an edge to split.
- **Terminal tabs** (portable-pty + xterm.js), and *Open in terminal* to
  continue a session in the CLI's own TUI.
- **Statusline**: model, effort, permission mode, folder, git branch,
  context %, cost, lines changed, 5-hour and weekly limits. Optionally runs
  your own Claude `statusLine.command` with the same input Claude Code gives it.
- **Progress panel**: turn state, elapsed time, current tool, plan.
- **Accounts** (a Rust port of mamadoo): Claude and Codex accounts with
  plan, limits, token usage, share/link to the central home, add with the
  tool's own login, rename, archive, delete. File-compatible with
  `account_manager.py`, so both can be used side by side.
- **History search** (Cmd/Ctrl+K): SQLite FTS5 over past prompts and
  replies of both tools, with Persian normalization (ي/ك → ی/ک, ZWNJ,
  Persian digits).
- **Attachments**: paste or drop images; other files are passed by path.
- **Notifications** when a turn ends or a permission waits while the window
  is in the background; the dock badge counts pending approvals.

## Requirements

- macOS 11+, Linux (WebKitGTK 4.1) or Windows 10+.
- The `claude` and/or `codex` CLI installed and logged in. Glaude looks for
  `CLAUDE_BIN` / `CODEX_BIN`, then the usual install locations, then your
  login shell's `PATH`.

Glaude never reads credentials or the keychain; the CLIs do their own auth.
API-key environment variables are removed before the CLIs start so they use
your subscription login, and Glaude warns if a session still bills an API key.

## Develop

```sh
cd ui && bun install && cd ..
bunx @tauri-apps/cli@2 dev
```

The UI also runs in a plain browser with a mock backend (`cd ui && bun run dev`,
then open http://localhost:1420), which is how the UI tests run.

Checks:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cd ui && bun run check && bun run test
```

Live tests against the real CLIs cost a little quota and are opt-in:

```sh
cargo test -p glaude-claude --test live -- --ignored --nocapture
cargo test -p glaude-codex --test live -- --ignored --nocapture
```

## Build

```sh
bunx @tauri-apps/cli@2 build
```

Bundles land in `target/release/bundle/` (`.app` and `.dmg` on macOS,
`.deb` and `.AppImage` on Linux, `.msi` and `.exe` on Windows).
`.github/workflows/ci.yml` builds all three platforms.

## Layout

```
crates/
  glaude-core      shared model, markdown → HTML blocks, bidi, diffs, ANSI, env/PATH loading
  glaude-claude    Claude Code stream-json driver, translator, transcript reader
  glaude-codex     codex app-server JSON-RPC client and thread translator
  glaude-accounts  mamadoo port: discovery, usage scan, limits, share, launch
  glaude-search    FTS5 history index
src-tauri/         Tauri app: commands, event batching, PTYs, settings
ui/                Svelte 5 + Tailwind + shadcn-svelte, xterm.js
docs/              protocol notes and the mamadoo port spec
fixtures/          recorded, scrubbed CLI sessions for tests
```

Rust does the heavy work (processes, parsing, rendering, highlighting,
diffs, scans, search); the webview places ready-made HTML blocks.
