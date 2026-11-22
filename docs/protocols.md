# Agent protocols

Notes on driving the installed CLIs. Verified against Claude Code 2.1.329 and
codex-cli 0.171.0 (2026-11-22). Re-check when either CLI updates.

## Claude Code (stream-json over stdio)

### Launch

```
claude -p --input-format stream-json --output-format stream-json --verbose \
  --include-partial-messages --replay-user-messages \
  --permission-prompt-tool stdio --permission-mode <mode> \
  [--model <m>] [--effort low|medium|high|xhigh|max] \
  [--resume <sid> [--fork-session]] | [--session-id <new-uuid>]
```

- cwd = project folder.
- `--input-format stream-json` requires `--print`, and stream-json output
  requires `--verbose`.
- Never use `--bare`: it never reads OAuth/keychain, so subscription logins break.
- Permission modes: `default` (the CLI's help calls it "manual"),
  `acceptEdits`, `plan`, `auto`, `dontAsk`, `bypassPermissions`.

### Environment

- Remove `ANTHROPIC_API_KEY` and `ANTHROPIC_AUTH_TOKEN`. Otherwise the CLI
  bills the key instead of the login; check `init.apiKeySource`.
- Remove `CLAUDE_CODE_*`, `CLAUDE_PID`, `NODE_OPTIONS` and `CLAUDECODE`. A
  parent Claude session sets these, and they confuse the child.
- `CLAUDE_CONFIG_DIR` selects the account:
  - It must be an exact absolute path with no `~` and no trailing slash.
  - The keychain item is `Claude Code-credentials-<sha256(dir)[0:8]>`, so a
    different spelling of the path looks logged out.
  - The `default` account (`~/.claude`) must run **without** the variable.
  - On Linux and Windows, credentials live in `<dir>/.credentials.json`.

### stdin (NDJSON)

- User message:
  `{"type":"user","session_id":"","parent_tool_use_id":null,"message":{"role":"user","content":[{"type":"text","text":"…"},{"type":"image","source":{"type":"base64","media_type":"image/png","data":"…"}}]}}`
- Keep stdin open; EOF ends the session after the queue drains.
- Slash commands are plain user text (`/compact`). Only the commands in
  `init.slash_commands` work.

### stdout events

| type | notes |
|---|---|
| `system/init` | `session_id, cwd, model, tools, mcp_servers, permissionMode, slash_commands, apiKeySource, claude_code_version` |
| `stream_event` | raw API stream events: `message_start`, `content_block_start/delta/stop` (text_delta, thinking_delta, input_json_delta), `message_delta`. Has `parent_tool_use_id` |
| `assistant` | full message; content = text / thinking / tool_use blocks. `parent_tool_use_id` set for subagent output |
| `user` | `tool_result` blocks (`tool_use_id, content, is_error`); replayed user input has `isReplay:true` |
| `result` | `subtype` success / error_during_execution / error_max_turns; `total_cost_usd, usage, modelUsage{m:{contextWindow…}}, duration_ms, num_turns, permission_denials` |
| `system/status` | `status: compacting / requesting / null`, `permissionMode` |
| `system/compact_boundary` | `compact_metadata{trigger, pre_tokens}` |
| `system/api_retry` | `attempt, max_retries, retry_delay_ms, error` |
| `system/session_state_changed` | `state: idle / running / requires_action` |
| `rate_limit_event` | `rate_limit_info{status, rateLimitType: five_hour / seven_day, utilization, resetsAt}` |
| `conversation_reset` | after `/clear`; `new_conversation_id` |
| `keep_alive` | ignore |

### Control protocol

- Envelope: `{"type":"control_request","request_id","request":{"subtype",…}}`.
- Reply success: `{"type":"control_response","response":{"subtype":"success","request_id","response":{…}}}`.
- Reply error: `{"type":"control_response","response":{"subtype":"error","request_id","error":"…"}}`.

Host to CLI requests:
- **Setup:** `initialize` returns `commands`, `models`, `account`.
- **Turn control:** `interrupt`.
- **Settings:** `set_permission_mode{mode}`, `set_model{model}`.
- **Usage:** `get_context_usage`, `get_usage`. `get_usage` returns
  `rate_limits{five_hour,seven_day}`, `total_cost_usd` and lines added/removed.
- **Other:** `file_suggestions`, `rename_session`.

CLI to host requests:
- `can_use_tool{tool_name, input, tool_use_id, permission_suggestions, blocked_path, decision_reason, title, description}`. Answer:
  - allow: `{"behavior":"allow","updatedInput":{…},"updatedPermissions":[…]}`
  - deny: `{"behavior":"deny","message":"…","interrupt":false}`
  - `control_cancel_request{request_id}` withdraws a pending prompt.

### Transcripts

- Path: `<config>/projects/<enc(cwd)>/<sessionId>.jsonl`, where `enc` replaces
  every non-alphanumeric character with `-`.
- Lines: `user`, `assistant` (one content block per line; merge by
  `message.id`), `system`.
- Metadata lines (`ai-title`, `custom-title`, `last-prompt`, …) are appended
  again and again, so always take the last one.
- Title = customTitle, then `<sid>/custom-title.json`, then aiTitle, then
  lastPrompt, then the first prompt.
- Skip files whose first line is `isSidechain`.

## Codex (`codex app-server`, JSON-RPC over stdio)

### Process and handshake

- Run one process per `CODEX_HOME` (account). Set `CODEX_HOME` explicitly,
  also for the default account (`~/.codex`).
- Messages are NDJSON JSON-RPC **without** the `"jsonrpc"` field. Sort each
  line by its keys:
  - `id` + `method`: a server request; you must answer it.
  - `id` + `result`/`error`: a response to your request.
  - `method` only: a notification.
- Handshake: `initialize{clientInfo:{name,title,version},capabilities:{experimentalApi:true,requestAttestation:false}}`,
  then the notification `initialized`.

### Requests

- **Threads:** `thread/start{model?, cwd, approvalPolicy, sandbox}`,
  `thread/resume{threadId, excludeTurns}`, `thread/turns/list`,
  `thread/list{sortKey:"updated_at", cwd?}`.
- **Turns:** `turn/start{threadId, input:[{type:"text",text,text_elements:[]},{type:"localImage",path}], model?, effort?}`,
  `turn/interrupt{threadId, turnId}`, `turn/steer`.
- **Account:** `account/read`, `account/rateLimits/read`, `model/list`.

### Notifications

- **Turn and item lifecycle:** `turn/started`, `turn/completed{turn.status}`,
  `item/started`, `item/completed`.
- **Item types:** `agentMessage`, `reasoning`, `commandExecution`,
  `fileChange`, `mcpToolCall`, `webSearch`, `plan`, `contextCompaction`.
- **Deltas:** `item/agentMessage/delta`, `item/reasoning/summaryTextDelta`,
  `item/commandExecution/outputDelta`.
- **Plan, diff, usage:** `turn/plan/updated`, `turn/diff/updated`,
  `thread/tokenUsage/updated`, `account/rateLimits/updated`.
- **Errors:** `error{error, willRetry}`.

### Server requests (approvals)

- `item/commandExecution/requestApproval` and
  `item/fileChange/requestApproval`.
- Answer with `{decision: "accept" | "acceptForSession" | "decline" | "cancel"}`.
- Answer unknown requests with a JSON-RPC error.

### Rate limits

- In `RateLimitSnapshot`, each of `primary` and `secondary` is
  `{usedPercent, windowDurationMins, resetsAt}`.
- A window of `windowDurationMins >= 1440` is weekly; anything shorter is the
  5h window.

### Observed behaviour (0.171.0, recorded in `fixtures/codex/`)

- Type definitions for the installed version: `codex app-server generate-ts --out <dir>`.
  Regenerate after a Codex upgrade and diff `ThreadItem.ts`, `ClientRequest.ts`, `ServerRequest.ts`.
- A turn sends `item/started` for a `commandExecution` *before* the
  `item/commandExecution/requestApproval` server request; the approval's
  `itemId` points at it. `serverRequest/resolved` follows our answer.
- `command` is wrapped (`/bin/zsh -lc "..."`); `commandActions[0].command`
  holds the plain command and is what the UI shows.
- Answer approvals with `{"decision": "accept" | "acceptForSession" | "decline" | "cancel"}`.
  Legacy `execCommandApproval` / `applyPatchApproval` are not used by v2 threads.
- `fileChange.changes[].diff` is a unified diff for updates and the plain
  file content for `add`/`delete`; Glaude turns the latter into a diff.
- `thread/tokenUsage/updated.tokenUsage.last.totalTokens` is the current
  context use, `modelContextWindow` the window.
- `account/rateLimits/updated` is not tied to a thread; it is broadcast to
  every open thread of that app-server. `usedPercent` is a percent;
  windows of 1440 minutes or more are treated as weekly.
- `thread/list` defaults to interactive sources; Glaude asks for
  `cli`, `vscode`, `appServer` and skips `ephemeral` threads.
- Resume: `thread/resume {excludeTurns: true}`, then page
  `thread/turns/list {sortDirection: "asc", itemsView: "full"}`.
- A cold `codex app-server` start plus `thread/list` takes several seconds
  (MCP servers start on the first thread), so the sidebar loads Codex
  sessions separately from Claude's.
- Unhandled server requests (`item/tool/requestUserInput`,
  `mcpServer/elicitation/request`, `item/permissions/requestApproval`) get a
  JSON-RPC error reply for now.

### Permission modes (UI value → approval policy / sandbox)

| UI | `approvalPolicy` | `sandbox` (thread) / `sandboxPolicy` (turn) |
| --- | --- | --- |
| `full` (default, like mamadoo `--yolo`) | `never` | `danger-full-access` |
| `on-request` | `on-request` | `workspace-write` |
| `untrusted` | `untrusted` | `workspace-write` |
| `read-only` | `on-request` | `read-only` |
