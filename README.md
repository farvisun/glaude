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
- **Agents keep working when Glaude is closed**: a background host
  (`glaude --host`) owns the `claude`/`codex` processes. Tabs come back on
  the next start, attached live if their agent is still running. With no
  window open, idle conversations stop at once; a running turn, background
  task, queued message or scheduled resume keeps going, and the host exits a
  minute after the last one ends. A desktop notification says when an agent
  finishes or needs an answer while no window is open.
- **Queue and steer**: Enter while a turn runs queues the message (edit,
  remove or send now above the composer); Cmd/Ctrl+Enter sends it into the
  running turn (Codex `turn/steer`).
- **Resume at the limit reset**: a turn that hits the usage limit shows when
  it resets and can continue on its own then (or in 1, 3 or 5 hours), even
  with Glaude closed.
- **Composer**: ArrowUp/ArrowDown walk earlier prompts, Cmd/Ctrl+S stashes the
  draft (restore it from the stash menu), select text in a reply to quote it.
- **Side chats**: `/btw <question>` or *Side chat* in the tab menu opens a fork
  of the conversation beside it; *Add to chat* quotes its answer back.
- **File links**: paths in replies that exist (`src/main.rs:12`, relative
  links, `file://`) open in your editor at the line (VS Code, Cursor, Zed or
  the default app, in Settings), in a read-only viewer tab, in Finder, or in
  a terminal there. Programs are never started from a link.
- **Changes tab** (click the branch in the statusline, or Cmd/Ctrl+Shift+G):
  uncommitted changes, the branch against its base, or one commit; filter by
  path or text; leave notes on lines and send them to a conversation as a
  review; commit (message written by a quick Haiku call if you like), push,
  and create a pull request with `gh`.
- **Worktree conversations**: start a conversation on its own branch in a new
  git worktree. `.glaude.json` in the project can set up and clean up
  worktrees and add actions (Dev, Test, ...) that run in a terminal tab:

  ```json
  {
    "worktree": { "setup": "bun install", "cleanup": "docker compose down" },
    "actions": [{ "name": "Dev", "command": "bun run dev" }]
  }
  ```
- **Terminal drawer** under each conversation (Ctrl+\`), in its folder.
- **Triage**: pin, snooze (an hour, this evening, tomorrow, next week) or
  archive sessions in the sidebar, with Undo; Cmd/Ctrl+K ranks by how often
  and how recently you opened them.
- **Sidebar, tabs and splits**: up to eight panes; drag a tab to an edge, or
  right-click a tab or pane to split it. Editor-style tabs show whether an
  agent is working, needs you, or finished while you were elsewhere; they
  scroll sideways with a list of all tabs when they don't fit.
- **macOS menu bar**: Glaude, File, Edit, View, Conversation, Window and Help
  menus with every action and its shortcut, and a status icon listing the
  agents that are working or waiting for you and what each is doing (it stays
  when the window is closed; Cmd+Q quits).
- **Live agent status**: the sidebar lists agents that need you, are working
  (with their current step, e.g. "Editing App.svelte · 1m 20s") or finished
  while you were elsewhere. The **Agents board** (⌥⌘A) shows every open agent
  by state; approvals and questions can be answered there, and ⇧⌘A jumps to
  the next agent waiting for you. Reminders fire when an agent has waited for
  a while or a command runs long.
- **Conversation view**: consecutive tool calls fold into one line ("Read 6
  files · Ran 3 commands") with the edited files; each turn ends with its
  time, cost, tokens and changed files, and actions to copy, retry, fork from
  there, or ask the other tool to review. ⌘F finds in the conversation, an
  outline of your prompts sits at the edge, and code blocks have copy, wrap
  and fold.
- **Cross-review**: Codex reviews Claude's uncommitted changes (or the other
  way round) read-only in a pane beside it; "Send findings back" puts the
  review in the original conversation's message box.
- **Command palette** (⇧⌘P) for every action, open agent and saved prompt.
  **Saved prompts** with fill-in parts like `{{file}}` are used by typing
  `/name` in a message.
- **Usage**: cost, tokens, turns and agent time per day, project and
  conversation for turns run in Glaude, plus every account's 5-hour and
  weekly limits. The progress panel shows a timeline of the latest turn.
- **Settings** (Cmd+,): appearance (theme, palette, interface size, density,
  message width, code font), defaults for new conversations (tool, account,
  model, effort, permissions, send key), background and alerts (keep agents
  working after quitting, idle stop time, which notifications, sound),
  terminal (font, size, cursor, shell) and a list of keyboard shortcuts.
  Searchable; a bottom sheet of sections on narrow windows.
- **Terminal tabs** (portable-pty + xterm.js), and *Open in terminal* to
  continue a session in the CLI's own TUI.
- **Statusline**: model, effort, permission mode, folder, git branch,
  context %, cost, lines changed, 5-hour and weekly limits. Optionally runs
  your own Claude `statusLine.command` with the same input Claude Code gives it.
- **Progress panel**: turn state, elapsed time, current tool, plan, and the
  conversation's tasks.
- **Claude agents and workflows**: each subagent is a card with its type,
  live status, usage, steps and report; Workflow runs show their phases and
  agents; background shells have an output viewer. Any task can be stopped.
- **Questions and plans**: Claude's questions become a form (single or
  multiple choice, other answer, previews); a finished plan can be approved
  (auto-accept edits or ask) or sent back with feedback. Shift+Tab cycles the
  permission mode, plan mode included.
- **Rewind**: from any of your messages, undo Claude's file edits since then,
  fork the conversation from there, or both.
- **Session panel** (Claude): context usage, MCP servers (reconnect, turn
  off), skills, agents, hooks and recent hook runs, memory files, output
  style. `@` in the composer mentions agents and files; a new conversation
  can run as a custom agent.
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

To build and install on this Mac:

```sh
scripts/install-macos.sh             # build, then install to /Applications
scripts/install-macos.sh --no-build  # install the last build
```

The script moves the bundle out of `target/` (a copy left there would show
up as a second Glaude in Spotlight), signs it ad hoc and registers it with
Launch Services. Quit Glaude first.

## Layout

```
crates/
  glaude-core      shared model, markdown → HTML blocks, bidi, diffs, ANSI, env/PATH loading
  glaude-claude    Claude Code stream-json driver, translator, transcript reader
  glaude-codex     codex app-server JSON-RPC client and thread translator
  glaude-accounts  mamadoo port: discovery, usage scan, limits, share, launch
  glaude-search    FTS5 history index
src-tauri/         Tauri app: commands, PTYs, settings, git, file links;
                   `host/` is the background agent host (`glaude --host`)
ui/                Svelte 5 + Tailwind + shadcn-svelte, xterm.js
docs/              protocol notes and the mamadoo port spec
fixtures/          recorded, scrubbed CLI sessions for tests
```

Rust does the heavy work (processes, parsing, rendering, highlighting,
diffs, scans, search); the webview places ready-made HTML blocks.
