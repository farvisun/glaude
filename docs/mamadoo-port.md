# mamadoo port (`crates/glaude-accounts`)

This is the port of `~/Projects/configs/account_manager.py`, cited below as
`am:<line>`. The Rust code must stay **file-compatible** with the Python tool
so the two can run side by side.

## Paths and environment

| What | Where |
|---|---|
| Root | `MAMADOO_DIR`, default `~/.claude_multi` |
| Codex config | `MAMADOO_CODEX_CONFIG`, default `$ROOT/.mamadoo-codex.json` |
| Binaries | `CLAUDE_BIN`, `CODEX_BIN` |
| Claude config | `$ROOT/.mamadoo.json` holds `{archived:[str], ui?:{sort:"name"\|"recent"\|"tokens"}}`; `$ROOT/.active` holds the active name plus `\n` |
| Codex config | `{archived:[str], active?:str}` |
| Claude account homes | `$ROOT/<name>/`, used as `CLAUDE_CONFIG_DIR`. `default` is `~/.claude`, protected, and keeps its state in `$ROOT/.default/` |
| Codex account homes | `~/.codex_<name>/`, used as `CODEX_HOME`. `default` is `~/.codex`, protected, and its state dir is the home itself |
| `<state>/.meta.json` | `{created_at, last_used, session_count, synced_mcp?}` |
| `<state>/.usage.json` (Claude) | `total_input_tokens, total_output_tokens, total_cache_read_input_tokens, total_cache_creation_input_tokens, total_tokens, usage_record_count, session_count, tokens_today, tokens_7d, models{top5}, latest_activity, last_updated` |
| `<state>/.usage.json` (Codex) | `total_tokens, session_count, tokens_7d, tokens_today, latest_activity, last_session{…}, total_*_tokens, cost_usd:0, limit_5h{used_percent, window_minutes, resets_at(epoch s)}, limit_weekly, limits_as_of, plan_type, last_updated` |
| Usage index | `$ROOT/.usage_index.shared.json`, used when `<acct>/projects` is a symlink or the account is default; otherwise `<acct>/.usage_index.json`. Format: `{version:2, files:{"projects/…jsonl":{offset,in,out,cr,cw,msgs,days{YYYY-MM-DD:n},models{},last_ts,recent[≤24],size,mtime(ns int)}}}`, written as compact JSON |
| Backups | `~/.agent-accounts-backup/YYYYmmdd-HHMMSS/<tool>/<acct>/…`, timestamp in local time |

**Formats and conventions**
- Atomic write: write to `.<name>.<pid>.tmp` in the same folder, then rename. Indent is 2, except the usage index (compact).
- Reads are lenient: an unreadable or invalid file is treated as `{}`.
- Timestamps:
  - Stored as ISO `%Y-%m-%dT%H:%M:%SZ` in UTC.
  - `parse_ts` accepts ISO strings, epoch seconds or epoch milliseconds. Any number above 1e10 is milliseconds.
  - An ISO string with no timezone is UTC.

**Account names**
- `normalize_name`: trim, turn whitespace into `_`, then turn anything outside `[A-Za-z0-9._-]` into `_`.
- New names must match `^[A-Za-z0-9][A-Za-z0-9._-]*$`. Reject `.`, `..` and `default`.

## Profile (plan, email, limits)

**Claude: read `<home>/.claude.json`**
- For the `default` account the file is `~/.claude.json`.
- Fields read: `oauthAccount{emailAddress, organizationName, userRateLimitTier, organizationRateLimitTier, seatTier, billingType}` and `cachedUsageUtilization{fetchedAtMs, utilization{five_hour,seven_day:{utilization:%, resets_at}}}`.
- Plan (`am:575`), checked in this order:
  1. `tier = user or org tier`. If it matches `max_(\d+x)`, the plan is `max 5x` (etc.). Otherwise, if it contains `claude_ai` or `pro`, the plan is `pro`.
  2. If `seatTier` contains `team`, the plan becomes `team <p>`. If it contains `enterprise`, it becomes `ent <p>`.
  3. If there is still no plan, use the part of `billingType` before the first `_`.
- Glaude also reads live limits from open sessions (`get_usage`, `rate_limit_event`).
- **No keychain reads.**

**Codex**
- Decode the payload of `auth.json` `tokens.id_token` (base64url, no signature check).
- email = `claims.email`, or `(api key)` if `OPENAI_API_KEY` is set.
- plan = `claims["https://api.openai.com/auth"].chatgpt_plan_type`, falling back to `usage.plan_type`.
- Live data comes from `account/rateLimits/read`.

**Display rule**
- left = clamp(100 − used).
- If `resets_at` is in the past, left = 100.
- Colour tiers: above 50, above 20, otherwise low.

## Usage scan

**Claude** (`am:589`, `am:747`)
1. Glob `<acct>/projects/**/*.jsonl` recursively. Follow symlinks and skip dot entries. The key is the path relative to the account, with symlinks left unresolved.
2. If `size` and `mtime_ns` are unchanged, skip the file. If `size < offset`, rescan from 0. Otherwise resume from `offset`.
3. Read line by line. Stop at a final line with no `\n`, leaving `offset` at its start.
4. Pre-filter: the line must contain both `"usage"` and `"assistant"`. Parse it and require `message.usage`.
5. Dedup on `"{message.id}:{requestId}"` against the last 24 keys.
6. Add up `input`, `output`, `cache_read` and `cache_creation` tokens.
   - `msgs` += 1.
   - `models[model]` += 1, unless the model is empty or starts with `<`.
   - `days[local date]` += the sum of the four counts.
   - `last_ts` = the max.
7. Save the index every 3 seconds or more during the scan, and at the end.
8. Aggregate:
   - `session_count` = files with `msgs > 0` that are not under `/subagents/`.
   - Today and 7d use **local calendar days**: today, and today − 6.

**Codex** (`am:1029`)
1. Open `<home>/state_5.sqlite` read-only.
2. If `threads.creator_account_id` exists, filter on the auth `account_id` (the default account also counts NULL).
   - Query `SUM(tokens_used)`, `COUNT(*)` and `MAX(updated_at)`.
   - 7d is **rolling** (`now − 7·86400`); today starts at local midnight.
3. Take up to 40 recent rollouts: from `rollout_path`, or else `sessions/*/*/*/*.jsonl` sorted in reverse.
4. Tail-read each file (256 KB chunks, at most 8 MB).
   - The first `token_count` with `info.total_token_usage` gives `last_session`.
   - The first `rate_limits` with a window gives the limits.
   - A window of 1440 minutes or more is weekly; anything shorter is 5h.

## Share (`am:440`)

**When an item counts as linked**
- An item is linked only if the account entry is a symlink whose `readlink` text equals the central absolute path exactly.
- An item that is missing on both sides needs nothing.

**Running check**
- An account counts as running when a claude/codex process has `CLAUDE_CONFIG_DIR=<home>` or `CODEX_HOME=<home>` in its environment.
- If the account is running and items are pending, refuse.

**Before sharing (Codex only)**
- Applies to `state_5.sqlite` (tables `threads`, `thread_spawn_edges`) and `goals_1.sqlite` (table `thread_goals`).
- Steps: `PRAGMA wal_checkpoint(TRUNCATE)`, then `ATTACH`, then `INSERT OR IGNORE`, rewriting `rollout_path` from `<acct>/` to `<central>/`.

**For each pending item**
- If the account entry is a symlink, unlink it. If it is a real entry, call `merge_into(item, central/item, backup/item, deep = item in data_dirs)`.
- Move `-wal` and `-shm` files into the backup too.
- Then symlink to the absolute central path.

**`merge_into`**
- If the central entry is missing, move the account entry there.
- **Folders on both sides:**
  - If `deep` is false, stash the account's child (central wins).
  - Otherwise recurse.
  - Afterwards, rmdir the account folder.
- **Files on both sides:**
  - For `history.jsonl` and `session_index.jsonl`, append the missing lines (de-duplicated, stably sorted by `timestamp`/`ts`/`updated_at`), then stash the account copy.
  - If `deep` is true, the file is `.jsonl` and the account copy is larger: stash central as `<name>.central` and move the account copy in.
  - Otherwise stash the account copy.

**After sharing**
- **Claude:**
  - Applies only if `.claude.json` has `oauthAccount`.
  - Sync `mcpServers`, minus the old `synced_mcp` entries that are gone from central.
  - For each central project, copy `mcpServers`, `enabledMcpjsonServers`, `disabledMcpjsonServers`, `hasTrustDialogAccepted` and `allowedTools`.
  - Store `synced_mcp` in `.meta.json`.
  - Write the file only if it changed, then `chmod 600`.
- **Codex:** `UPDATE threads SET rollout_path = '<central>/' || …` wherever it starts with `<acct>/`.

**Windows**
- Try a symlink first. Fall back to a directory junction for folders.
- For files, ask the user to turn on Developer Mode.

## Launch (`am:417`)

1. `mkdir -p` the home, then share (errors never block), then bump the meta, then set the account active.
2. Drop foreign flags:
   - For Claude: `--yolo`, `--dangerously-bypass-approvals-and-sandbox`.
   - For Codex: `--dangerously-skip-permissions`, `--allow-dangerously-skip-permissions`, `--yolo`.
3. Environment:
   - Claude: remove `CLAUDE_CONFIG_DIR`, then set it unless the account is `default`.
   - Codex: always set `CODEX_HOME`.

## Status and verification

Ported in `crates/glaude-accounts`. `examples/dump.rs` runs the same steps
as the Python tool on any HOME (`list`, usage refresh, `share`).

Checked against `account_manager.py` on a scratch HOME (subset of real
transcripts and Codex state, fake auth files):

- Usage: `.usage.json` totals, sessions, today/7d, models, limits, plans and
  emails match for every account; the scan index (`.usage_index*.json`)
  matches entry by entry (offsets, day buckets, dedup keys, mtimes).
- Share: on two identical homes, Python and Rust produce the same log
  lines, the same tree (links, merged files, backups), the same merged
  thread rows and the same `.claude.json` (mode 0600).

Known gaps:

- Running-process detection uses `ps eww`, so it works on macOS and Linux
  only; on Windows nothing counts as running (share never refuses there).
- Model ties in the top-5 list may come out in a different order than in
  Python (both sort by count; insertion order differs).

## Not ported to the UI

`clear` is left out: in Python, `clear` on the Claude side wipes `$ROOT`, which includes the Codex config.
