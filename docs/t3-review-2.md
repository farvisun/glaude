# T3 Code review 2: features Glaude does not have yet

Reviewed: T3 Code (Beta) **0.1.11** (stable, commit `59651c1761f6`, released
2026-11-20), installed at `/Applications/T3 Code (Beta).app`.
Date: 2026-11-22.

How: the app is Electron. I extracted `app.asar` and read the English UI
strings (`apps/server/dist/client/assets/en-*.js`, about 9,200 strings), the
settings search keywords, the keybinding command IDs and the route chunk names
(`_chat.settings.*`, `_chat.issues`, `_chat.pull-requests`). I also read the
repo's `docs/user/*.md` at tag `v0.1.11` and the release notes for v0.1.5 to
v0.1.11. I did not launch the app; the strings and docs were enough.

Already ported from the first review (see `plan-t3-features.md`) and left out
here: agent host, queue and steer, resume at limit reset, ArrowUp recall,
prompt stash, quote, side chats, file links, terminal drawer, Changes tab,
commit/push/PR, worktree conversations with `.glaude.json`, pin/snooze/archive.

Effort is a rough guess for Glaude: **small** (a day or less), **medium**
(a few days), **large** (a week or more).

Where a source says "strings", it is the English UI string file. "Settings
keywords" are the search keywords T3 attaches to each setting.

## Conversation

- **Edit from here** (medium). Rewinds the same thread to before a message and
  puts that prompt back in the composer to edit and resend. It can keep the
  files or restore them. Glaude's rewind forks instead of editing in place.
  Seen: `composer.md` "Edit an earlier prompt"; strings "Revert and keep
  changes", "Revert files too".
- **Suggested next prompt** (medium). After a Claude or Codex turn, the empty
  composer shows a guessed next message as ghost text. Tab accepts it, Escape
  dismisses it. Seen: `composer.md`; settings keywords "tab autocomplete
  predict next message claude codex ghost text".
- **Goals** (medium). `/goal <what done means>` keeps the agent working across
  turns until the goal is met. The thread shows a Goal badge and progress.
  It supports `/goal pause`, `resume` and `clear`. Seen: `composer.md`
  "Goals".
- **Reference another conversation with `@`** (medium). Typing `@` and part of
  a title attaches another thread as a chip. The agent reads that transcript
  only when it needs it. Dragging a thread from the sidebar onto the composer
  does the same. Seen: `composer.md` "Context in your message".
- **Pick a pull request with `#`** (small). `#` lists recent PRs of the repo
  and inserts one as a chip with its title and branches. Seen: `composer.md`.
- **`/clear` and Restart agent session** (small). `/clear` starts a new CLI
  session in the same tab and marks "New session" in the timeline. *Restart
  agent session* reloads skills, plugins and MCP config without losing the
  conversation. Seen: `composer.md` "Commands and skills".
- **Large paste becomes a file** (small). Pasting 32 KiB or more attaches it as
  a text file instead of filling the message. Seen: `composer.md`.
- **Visual replies and MCP Apps** (large). The agent can answer with a
  self-contained HTML page (chart, table, mockup) shown in the thread. MCP
  tools that return an MCP App render it inline. Seen: `html-renders.md`.
- **Math and Mermaid** (small to medium). LaTeX in replies is typeset (KaTeX),
  and Mermaid diagrams render. Seen: settings keywords "latex tex katex";
  assets `katex`, `MermaidViewer`, `MathTypeset`.
- **Rendered file and attachment previews** (medium). Markdown, HTML and CSV
  render, with a source toggle. Images, video, PDF and fonts have viewers.
  Office files and notebooks too (large). Glaude's file tab shows highlighted
  text only. Seen: `composer.md` "Attached files"; assets `PdfViewer`,
  `OfficeViewer`, `Spreadsheet`, `NotebookPreview`.
- **Hide tool calls** (small). A "final answers only" mode, or *Hide
  successful*, which shows a call only while it runs or if it fails. Seen:
  `activity-log.md`; settings keywords "quiet chat zen final answers only".
- **Model switch markers** (small). The timeline shows where the model or
  effort changed. Seen: v0.1.7 notes "Model switch history".
- **Plan and table actions** (small). *Save plan to workspace*, *Copy plan*,
  and *Copy as CSV* on tables. Seen: strings.

## Agents and providers

- **More providers** (large). OpenCode, Cursor, Antigravity (Gemini), Kiro,
  Pi, Muse Code, fx, Grok Build, and any ACP agent. One generic ACP driver
  would cover several of them. Seen: `docs/user/providers-*.md`,
  `providers-acp.md`.
- **Switch tool mid-conversation** (medium to large). Continue a Claude
  conversation in Codex or the other way round. T3 sends a trimmed copy of
  the history ("portable handoff"). Seen: `portable-handoffs.md`; README
  "Change models in the middle of a thread".
- **Agents delegate to other agents** (large). T3 gives agents MCP tools to
  start a child task on another provider, check it and cancel it. The child's
  approvals show in the parent. Seen: strings "Delegate a child task", "Get
  delegated task status"; v0.1.7 "Child approvals".
- **Same prompt to several models** (medium). Shift-click models in a new
  thread's picker. Each one gets its own conversation and worktree. Seen:
  `thread-sidebar.md` "Start in the background".
- **Start in the background** (small). Cmd+Enter in a new conversation starts
  it and opens a fresh draft right away. Seen: `thread-sidebar.md`; command
  `composer.sendBackground`.
- **Fallback when out of usage** (small to medium). New conversations avoid a
  provider or account that just hit its limit and use a fallback model.
  Seen: settings keywords "fallback agent provider out of usage new threads";
  strings "Fallback model"; v0.1.5 notes.
- **Custom models, launch arguments and env per account** (small). Add an
  unlisted model; give an instance extra CLI args (for example `--chrome`)
  or env vars (OpenRouter, routers). Seen: `composer.md` "Custom models";
  `providers-claude.md`.
- **Claude compaction controls** (small). An auto-compact threshold per
  instance, and an optional compact of idle threads before the prompt cache
  expires. Seen: `providers-claude.md`; strings "Before the cache expires
  (beta)".
- **Update the CLIs from the app** (small). Shows installed and newer versions
  of `claude` and `codex` and updates them. Seen: settings keywords
  "installed cli versions newer available".
- **Native resume** (small). *Resume* continues a stopped or limited turn
  without sending a "continue" message, where the CLI supports it. Seen:
  v0.1.10 notes; `thread-sidebar.md` "Inspect agent work".

## Git and worktrees

- **Worktree options** (small to medium). Pick the base branch, start from the
  latest `origin` branch, set a branch prefix (static, or `feat/`/`fix/`
  chosen by a model), init submodules, and *New thread in this worktree*.
  Glaude always branches from HEAD. Seen: settings keywords; `project-settings.md`.
- **Move a conversation into a worktree** (medium). *Hand off thread to a git
  worktree* moves running work out of the main checkout. Seen: strings.
- **Worktree cleanup** (medium). Policies delete worktrees of archived or
  deleted threads after N days, or worktrees with no commits beyond the
  default branch. Seen: `project-settings.md` "Storage cleanup".
- **Actions on archive** (small). `runOnSettle` (for example `cargo clean`)
  and `stopOnSettle` for project actions. Fits `.glaude.json`. Seen:
  `thread-sidebar.md` "Settle finished work".
- **Linked pull requests** (medium). Each conversation finds the PR for its
  branch, shows its state in the sidebar, and can link or unlink PRs. A
  merged PR can archive the thread. Seen: `thread-sidebar.md` "Link a pull
  request"; `source-control.md` "Linked pull requests".
- **Watch a pull request** (medium). The agent wakes when CI finishes, someone
  comments, or the branch conflicts. Seen: strings "Watch a pull request",
  "Watching: the agent wakes when checks finish, someone comments, or the
  branch conflicts".
- **Pull requests page** (large). Review PRs and comments, request reviewers,
  check out, merge (with a default merge method), stacked PRs, mark files as
  viewed, find in diff. *Review guides* (beta) add an AI-written tour.
  Seen: `source-control.md`; route `_chat.pull-requests`.
- **Issues page** (large; medium for GitHub only via `gh`). Lists issues from
  GitHub, GitLab, Linear, Azure DevOps and Forgejo. Hands an issue to an agent.
  Seen: v0.1.11 notes; `source-control.md` "Issues".
- **Diff options** (small). Hide whitespace changes, hide test files, open the
  diff on its own when a turn ends, and per-turn diffs that leave out files a
  pull or rebase brought in. Seen: strings; `source-control.md` "Review turn
  changes".
- **Commit and PR conventions** (small). Custom instructions for generated
  commit messages and PR text. The PR description also uses what the
  conversation did. Seen: settings keywords "repository conventions
  conventional commits custom instructions".
- **Keep the default branch current** (small). Fast-forward the main checkout
  from upstream in the background. Seen: `project-settings.md`.
- **Resolve conflicts in a thread** (small). A button hands a merge conflict
  to the agent. Seen: strings "Resolve the conflicts in this thread".
- **New, clone, publish** (medium). Create a project (git init, README, first
  commit), clone a repo into a ghq-style layout, publish to a host. Seen:
  `source-control.md` "Start, clone, or publish a project".

## Terminal

- **Terminal excerpt to chat** (small). Select terminal output and add it to
  the message as a chip. Seen: strings "Terminal excerpt, {label}",
  "Captured terminal output".
- **Detected local servers** (medium). The thread details list servers that
  its terminals or agent started, with ports. Open one in a browser, or stop
  it. Seen: `thread-sidebar.md` "Inspect agent work".
- **Terminals survive restarts** (medium). Scrollback (5,000 lines) and shells
  live on the server. In Glaude, shells belong to the app window, so moving
  them to the host would do the same. Seen: `terminal.md`.
- **Confirm before closing a busy terminal** (small). Seen: strings "Ask
  before closing terminals and stopping their running processes."

## Projects and sidebar

- **Conversations without a project** (small to medium). They run in a dated
  scratch folder (`~/.t3/scratch/...`). *Turn into project* and *Initialize
  Git* promote them later. Seen: `thread-sidebar.md` "Start without a project".
- **Settle and auto-archive** (medium). Threads idle for 7 days, or whose PR
  merged, leave the active list on their own. A per-thread "Never" option
  opts out. A *Working* shelf folds threads that are busy in the background.
  Seen: `thread-sidebar.md` "Settle finished work", "Fold working threads".
- **Manual order and drag** (medium). Drag to reorder, or drag between
  Pinned, Active and Settled. Multi-select for bulk snooze. Seen:
  `thread-sidebar.md` "Pin and reorder threads".
- **Regenerate title** (small). Also *Thread title instructions* per project.
  Seen: `thread-sidebar.md`; command `thread.regenerateTitle`.
- **Snooze extras** (small). Editable snooze presets, *Custom…* date and time,
  and *Snooze until reset* on a limit. Seen: `thread-sidebar.md` "Snooze until
  later".
- **Project pin, archive, icon and sort** (small to medium). Emoji, image or
  monogram icons; sort by manual, created or recent. Seen:
  `project-settings.md`; settings keywords.
- **Drop files on a sidebar row** (small). Opens that conversation with the
  files attached. Seen: `thread-sidebar.md`.
- **Links into the app** (small to medium). *Copy thread* and *Copy prompt
  link* give a URL that opens the conversation, or a new one with a prompt
  filled in. Seen: `prompt-links.md`.
- **Navigation** (small). Back and forward, reopen closed tab, jump to thread
  1 to 9, and F6 to move between areas. Seen: commands `navigation.back`,
  `view.reopenClosed`, `thread.jump.1`, `workspace.focusNext`;
  `keyboard-focus.md`.

## Settings and keybindings

- **Custom keybindings** (medium). A `keybindings.json` with `key`, `command`
  and `when` (for example `terminalFocus`), editable from Settings. Glaude's
  list is read-only. Seen: `keybindings.md`; strings "Open keybindings.json".
- **Custom themes** (medium). Theme editor, import of VS Code themes, custom
  CSS and a background image. Seen: `appearance.md` "Custom themes"; v0.1.10.
- **Default follow-up behavior** (small). Choose queue or steer as the
  default. Shortcuts steer the oldest queued message and edit the last one.
  Seen: `composer.md` "Send while the agent is working".
- **Text generation model** (small). Choose which model writes titles, commit
  messages, PR text and branch names. API-key providers are allowed too.
  Seen: `text-generation.md`.
- **App language** (medium). The whole UI is translated (10 languages). For
  Glaude, a Persian UI would be the natural one. Seen: v0.1.11 notes; assets
  `en`, `de`, `fr`, `ja`, ... locale files.
- **Display details** (small each). Separate fonts for prompt, code and
  terminal; 12/24-hour time; panel animation speed; window transparency; how
  reasoning effort is shown. Seen: settings keywords.
- **Quit confirmation** (small). Hold Cmd+Q or press it twice. Seen: settings
  keywords "confirmation desktop app exit direct hold double click".
- **More editors** (small). JetBrains IDEs, VSCodium, VS Code Insiders, Trae.
  Seen: editor labels in the desktop main bundle.

## Other

- **Keep the computer awake while agents work** (small). Seen: strings "Keep
  awake while agents work"; v0.1.10 notes.
- **Scheduled tasks** (medium). Run a prompt once at a time, every N
  minutes/hours/days, or on a webhook. Option for a new thread per run.
  Glaude's host could run them. Seen: route `_chat.settings.scheduled-tasks`;
  `project-settings.md`; v0.1.7 "One-time schedules".
- **Usage extras** (small to medium). Custom model prices, filters by provider
  and model, cache savings, and `C`/`T`/`L` keys to switch views. Seen:
  `usage.md`.
- **Diagnostics page** (medium). Logs, processes, and CPU and memory history.
  Seen: route `_chat.settings.diagnostics`; settings keywords "logs traces
  processes resource history".
- **Voice dictation** (large). Local speech models (Whisper and others) and
  optional cleanup of the transcript. Seen: `voice-input.md`.
- **SnapShots** (medium to large). A hotkey captures the front window and its
  accessibility text into the draft. Seen: `snap-shot.md`.
- **In-app browser** (large). Preview dev servers, annotate elements into the
  draft, device sizes, cookie import, record the page. Seen: route
  `_chat.settings.browser`; strings "Annotate preview"; `browser-import.md`.
- **MCP server and CLI for outside agents** (large). Other agents, scripts or
  `t3 thread start/send` can start and message conversations. Seen:
  `outside-agents.md`.
- **Secrets** (medium). 1Password references for credentials, and a private
  card when an agent asks for a token. Seen: `secrets.md`.
- **Auto-update with channels** (medium). Stable and nightly channels.
  Seen: `updating.md`; settings keywords "release channel stable latest
  nightly".

## Recommended first

1. **Keep awake while agents work.** Small, and it backs up Glaude's promise
   that agents keep working when the window is closed.
2. **Fallback account when a limit hits.** Glaude already knows every
   account's limits. Starting new work on an account with quota left is a
   short step.
3. **Same prompt to several models.** Claude and Codex in two worktrees, side
   by side, builds on worktrees and cross-review.
4. **Linked PR and PR watch.** Uses `gh`, which the Changes tab already needs.
   It wakes the agent when CI fails or a reviewer comments.
5. **Worktree options and cleanup.** Base branch, start from origin, branch
   prefix and cleanup finish the worktree feature.
6. **Terminal excerpt to chat and detected servers.** Small additions to the
   terminal panel with daily use.
7. **`@` another conversation.** Transcripts and FTS search already exist; the
   agent reads the file only when it needs to.
8. **Rendered previews and math/Mermaid.** Markdown, images, PDF and HTML in
   the file tab make file links more useful.

## Where Glaude is ahead, or features that don't apply

- RTL and bidi per block, the Estedad font and Persian search normalization:
  T3 has no RTL support and none of its 10 languages is RTL.
- Proxies per app, account, project and conversation: T3 has no proxy
  settings.
- A macOS menu bar status item listing agents: no tray code in T3's desktop
  bundle.
- Up to eight split panes of conversations. T3's strings show one thread
  with a right panel, plus "open this page in its own window".
- Cross-review with "Send findings back": T3's review guides cover PRs, not
  one tool reviewing the other's uncommitted work.
- A native Tauri app with Rust-side rendering, against T3's Electron bundle
  (about 280 MB extracted).
- Not relevant for Glaude: mobile apps, T3 Connect, remote environments, guest
  links, WSL backend, iOS and Android simulators, managed policies,
  telemetry, and the Windows service.
