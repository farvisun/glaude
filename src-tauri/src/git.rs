//! Git for the Changes tab: status, diffs (uncommitted, branch, one
//! commit), commit, push, pull requests through `gh`, worktrees for
//! conversations, and the project's `.glaude.json`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use glaude_core::model::FileDiff;
use serde::Serialize;
use serde_json::Value;
use tokio::io::AsyncWriteExt;

/// Diffs bigger than this are listed without their text.
const FILE_DIFF_LIMIT: usize = 512 * 1024;
const TOTAL_DIFF_LIMIT: usize = 24 << 20;
/// The empty tree: the base of a repository with no commits yet.
const EMPTY_TREE: &str = "4b825dc642cb6eb9a060e54bf8d69288fbee4904";

struct Out {
    ok: bool,
    stdout: String,
    stderr: String,
}

async fn run(
    program: &str,
    cwd: &Path,
    args: &[&str],
    stdin: Option<&str>,
    secs: u64,
) -> Result<Out, String> {
    let bin = glaude_core::env::which(program)
        .ok_or_else(|| format!("`{program}` was not found on your PATH"))?;
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(args)
        .current_dir(cwd)
        .env_clear()
        .envs(glaude_core::env::agent_env(&[
            ("GIT_TERMINAL_PROMPT", Some("0".into())),
            ("GH_PROMPT_DISABLED", Some("1".into())),
            ("NO_COLOR", Some("1".into())),
        ]))
        .stdin(if stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| format!("{program}: {e}"))?;
    if let (Some(text), Some(mut w)) = (stdin, child.stdin.take()) {
        let text = text.to_string();
        tokio::spawn(async move {
            let _ = w.write_all(text.as_bytes()).await;
        });
    }
    let out = tokio::time::timeout(Duration::from_secs(secs), child.wait_with_output())
        .await
        .map_err(|_| format!("{program} took too long"))?
        .map_err(|e| e.to_string())?;
    Ok(Out {
        ok: out.status.success(),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

async fn git(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let o = run("git", cwd, args, None, 60).await?;
    if o.ok {
        Ok(o.stdout)
    } else {
        Err(first_line(&o.stderr).unwrap_or("git failed").to_string())
    }
}

fn first_line(s: &str) -> Option<&str> {
    s.lines().map(str::trim).find(|l| !l.is_empty())
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitInfo {
    pub root: String,
    pub branch: String,
    /// The branch this one is compared with ("origin/main", "main").
    pub base: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    /// Changed or new files not yet committed.
    pub dirty: u32,
    pub has_commits: bool,
    /// A linked worktree (not the main checkout).
    pub worktree: bool,
    /// A worktree Glaude created (it can remove it).
    pub managed: bool,
    pub gh: bool,
}

async fn default_base(root: &Path) -> Option<String> {
    if let Ok(r) = git(
        root,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    )
    .await
    {
        let r = r.trim();
        if !r.is_empty() {
            return Some(r.to_string());
        }
    }
    for b in ["origin/main", "origin/master", "main", "master"] {
        if git(root, &["rev-parse", "--verify", "--quiet", b])
            .await
            .is_ok()
        {
            return Some(b.to_string());
        }
    }
    None
}

pub fn worktrees_dir() -> PathBuf {
    crate::host::data_dir().join("worktrees")
}

/// Inside the folder where Glaude puts its worktrees (symlinks resolved).
fn is_managed(p: &Path) -> bool {
    let dir = worktrees_dir();
    let dir = dir.canonicalize().unwrap_or(dir);
    let p = p.canonicalize().unwrap_or_else(|_| p.to_path_buf());
    p.starts_with(&dir) && p != dir
}

#[tauri::command]
pub async fn git_info(cwd: String) -> Result<GitInfo, String> {
    let cwd = PathBuf::from(cwd);
    let root = git(&cwd, &["rev-parse", "--show-toplevel"])
        .await
        .map_err(|_| "This folder is not in a git repository".to_string())?;
    let root = PathBuf::from(root.trim());
    let mut info = GitInfo {
        root: root.display().to_string(),
        ..Default::default()
    };
    info.has_commits = git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .await
        .is_ok();
    info.branch = git(&root, &["rev-parse", "--abbrev-ref", "HEAD"])
        .await
        .map(|b| b.trim().to_string())
        .unwrap_or_else(|_| glaude_core::git::branch(&root).unwrap_or_default());
    info.base = default_base(&root)
        .await
        .filter(|b| b.rsplit('/').next() != Some(info.branch.as_str()) || b.contains('/'));
    if let Some(b) = &info.base {
        // Comparing the default branch with itself shows nothing useful.
        if b == &info.branch {
            info.base = None;
        }
    }
    if let Ok(u) = git(
        &root,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .await
    {
        info.upstream = Some(u.trim().to_string());
        if let Ok(c) = git(
            &root,
            &["rev-list", "--left-right", "--count", "@{u}...HEAD"],
        )
        .await
        {
            let mut it = c.split_whitespace().filter_map(|v| v.parse().ok());
            info.behind = it.next().unwrap_or(0);
            info.ahead = it.next().unwrap_or(0);
        }
    }
    if let Ok(s) = git(
        &root,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )
    .await
    {
        info.dirty = s.split('\0').filter(|l| l.len() > 3).count() as u32;
    }
    let git_dir = git(&root, &["rev-parse", "--git-dir"])
        .await
        .unwrap_or_default();
    let common = git(&root, &["rev-parse", "--git-common-dir"])
        .await
        .unwrap_or_default();
    info.worktree = git_dir.trim() != common.trim();
    info.managed = info.worktree && is_managed(&root);
    info.gh = glaude_core::env::which("gh").is_some();
    Ok(info)
}

/// Split `git diff` output into one FileDiff per file.
pub fn parse_diff(text: &str) -> Vec<FileDiff> {
    let mut out = Vec::new();
    let mut total = 0usize;
    for chunk in text.split("\ndiff --git ").enumerate().map(|(i, c)| {
        if i == 0 {
            c.strip_prefix("diff --git ").unwrap_or(c)
        } else {
            c
        }
    }) {
        if chunk.trim().is_empty() {
            continue;
        }
        let header = chunk.lines().next().unwrap_or("");
        let mut path = header
            .split_once(" b/")
            .map(|(_, b)| b.to_string())
            .unwrap_or_default();
        let mut kind = "update";
        let mut binary = false;
        for line in chunk.lines().take(8) {
            if line.starts_with("new file mode") {
                kind = "add";
            } else if line.starts_with("deleted file mode") {
                kind = "delete";
            } else if let Some(p) = line.strip_prefix("rename to ") {
                path = p.to_string();
            } else if let Some(p) = line.strip_prefix("+++ b/") {
                path = p.to_string();
            } else if line.starts_with("Binary files") {
                binary = true;
            }
        }
        if path.is_empty() {
            continue;
        }
        let body = chunk.find("\n@@").map(|i| &chunk[i + 1..]).unwrap_or("");
        let (added, removed) = glaude_core::diff::count(body);
        let too_big = body.len() > FILE_DIFF_LIMIT || total > TOTAL_DIFF_LIMIT;
        total += body.len();
        let diff = if binary {
            "@@ -0,0 +0,0 @@ Binary file\n".to_string()
        } else if too_big {
            format!("@@ -0,0 +0,0 @@ Diff too large to show (+{added} −{removed})\n")
        } else {
            body.to_string()
        };
        out.push(FileDiff {
            path,
            kind: kind.into(),
            diff,
            added,
            removed,
        });
    }
    out
}

const DIFF_ARGS: &[&str] = &[
    "--no-color",
    "--no-ext-diff",
    "-M",
    "--src-prefix=a/",
    "--dst-prefix=b/",
];

/// Diffs of `source`: `uncommitted` (working tree and new files vs HEAD),
/// `branch` (commits since the base branch) or `commit` (one commit).
#[tauri::command]
pub async fn git_diff(
    cwd: String,
    source: String,
    rev: Option<String>,
) -> Result<Vec<FileDiff>, String> {
    let root = PathBuf::from(cwd);
    let text = match source.as_str() {
        "uncommitted" => {
            let head = if git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
                .await
                .is_ok()
            {
                "HEAD"
            } else {
                EMPTY_TREE
            };
            let mut args = vec!["diff"];
            args.extend(DIFF_ARGS);
            args.push(head);
            let mut text = git(&root, &args).await?;
            // New files git does not track yet.
            let untracked =
                git(&root, &["ls-files", "--others", "--exclude-standard", "-z"]).await?;
            for f in untracked.split('\0').filter(|f| !f.is_empty()).take(300) {
                let o = run(
                    "git",
                    &root,
                    &[
                        "diff",
                        "--no-color",
                        "--no-index",
                        "--src-prefix=a/",
                        "--dst-prefix=b/",
                        "--",
                        null_device(),
                        f,
                    ],
                    None,
                    30,
                )
                .await?;
                // --no-index exits 1 when the files differ.
                text.push('\n');
                text.push_str(&o.stdout);
            }
            text
        }
        "branch" => {
            let base = rev.ok_or("No base branch to compare with")?;
            let range = format!("{base}...HEAD");
            let mut args = vec!["diff"];
            args.extend(DIFF_ARGS);
            args.push(&range);
            git(&root, &args).await?
        }
        "commit" => {
            let rev = rev.ok_or("Which commit?")?;
            let mut args = vec!["show", "--format="];
            args.extend(DIFF_ARGS);
            args.push(&rev);
            git(&root, &args).await?
        }
        other => return Err(format!("Unknown source {other}")),
    };
    Ok(parse_diff(&text))
}

fn null_device() -> &'static str {
    if cfg!(windows) {
        "NUL"
    } else {
        "/dev/null"
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub sha: String,
    pub short: String,
    pub subject: String,
    pub author: String,
    pub date: String,
}

/// Commits since `base` (or the latest ones when there is no base).
#[tauri::command]
pub async fn git_log(
    cwd: String,
    base: Option<String>,
    limit: Option<u32>,
) -> Result<Vec<Commit>, String> {
    let root = PathBuf::from(cwd);
    let n = format!("-n{}", limit.unwrap_or(50).min(500));
    let range = base.map(|b| format!("{b}..HEAD"));
    let mut args = vec![
        "log",
        "--format=%H%x1f%h%x1f%s%x1f%an%x1f%aI%x1e",
        n.as_str(),
    ];
    if let Some(r) = &range {
        args.push(r);
    }
    let text = match git(&root, &args).await {
        Ok(t) => t,
        // No commits yet.
        Err(_)
            if git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
                .await
                .is_err() =>
        {
            String::new()
        }
        Err(e) => return Err(e),
    };
    Ok(text
        .split('\x1e')
        .filter_map(|rec| {
            let f: Vec<&str> = rec.trim().split('\x1f').collect();
            (f.len() == 5).then(|| Commit {
                sha: f[0].into(),
                short: f[1].into(),
                subject: f[2].into(),
                author: f[3].into(),
                date: f[4].into(),
            })
        })
        .collect())
}

/// Stage everything and commit. Returns the new commit's short id.
#[tauri::command]
pub async fn git_commit(cwd: String, message: String) -> Result<String, String> {
    let root = PathBuf::from(cwd);
    if message.trim().is_empty() {
        return Err("Write a commit message first".into());
    }
    git(&root, &["add", "-A"]).await?;
    let o = run("git", &root, &["commit", "-F", "-"], Some(&message), 120).await?;
    if !o.ok {
        let msg = first_line(&o.stderr)
            .or_else(|| first_line(&o.stdout))
            .unwrap_or("git commit failed");
        return Err(msg.to_string());
    }
    Ok(git(&root, &["rev-parse", "--short", "HEAD"])
        .await?
        .trim()
        .to_string())
}

#[tauri::command]
pub async fn git_push(cwd: String) -> Result<String, String> {
    let root = PathBuf::from(cwd);
    let has_upstream = git(&root, &["rev-parse", "--abbrev-ref", "@{u}"])
        .await
        .is_ok();
    let args: &[&str] = if has_upstream {
        &["push"]
    } else {
        &["push", "-u", "origin", "HEAD"]
    };
    let o = run("git", &root, args, None, 300).await?;
    if !o.ok {
        let lines: Vec<&str> = o.stderr.lines().filter(|l| !l.trim().is_empty()).collect();
        return Err(lines
            .iter()
            .rev()
            .take(3)
            .rev()
            .copied()
            .collect::<Vec<_>>()
            .join("\n"));
    }
    Ok(first_line(&o.stderr).unwrap_or("Pushed").to_string())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullRequest {
    pub url: String,
    pub number: u64,
    pub state: String,
    pub title: String,
}

/// The open pull request of the current branch, if any.
#[tauri::command]
pub async fn git_pr_status(cwd: String) -> Result<Option<PullRequest>, String> {
    let root = PathBuf::from(cwd);
    let o = run(
        "gh",
        &root,
        &["pr", "view", "--json", "url,number,state,title"],
        None,
        30,
    )
    .await?;
    if !o.ok {
        return Ok(None);
    }
    let v: Value = serde_json::from_str(&o.stdout).map_err(|e| e.to_string())?;
    Ok(Some(PullRequest {
        url: v["url"].as_str().unwrap_or_default().into(),
        number: v["number"].as_u64().unwrap_or(0),
        state: v["state"].as_str().unwrap_or_default().into(),
        title: v["title"].as_str().unwrap_or_default().into(),
    }))
}

/// Push the branch if needed and open a pull request. Returns its URL.
#[tauri::command]
pub async fn git_pr_create(
    cwd: String,
    title: String,
    body: String,
    base: Option<String>,
    draft: bool,
) -> Result<String, String> {
    let root = PathBuf::from(&cwd);
    if title.trim().is_empty() {
        return Err("Give the pull request a title".into());
    }
    let pushed = git(&root, &["rev-parse", "--abbrev-ref", "@{u}"])
        .await
        .is_ok()
        && git(&root, &["rev-list", "--count", "@{u}..HEAD"])
            .await
            .is_ok_and(|n| n.trim() == "0");
    if !pushed {
        git_push(cwd.clone()).await?;
    }
    let base = base.map(|b| b.strip_prefix("origin/").unwrap_or(&b).to_string());
    let mut args = vec![
        "pr",
        "create",
        "--title",
        title.as_str(),
        "--body-file",
        "-",
    ];
    if let Some(b) = &base {
        args.extend(["--base", b.as_str()]);
    }
    if draft {
        args.push("--draft");
    }
    let o = run("gh", &root, &args, Some(&body), 120).await?;
    if !o.ok {
        return Err(first_line(&o.stderr)
            .unwrap_or("gh pr create failed")
            .to_string());
    }
    Ok(o.stdout
        .lines()
        .rev()
        .find(|l| l.starts_with("https://"))
        .unwrap_or(o.stdout.trim())
        .to_string())
}

/// A commit message for the uncommitted changes, written by a one-shot
/// `claude -p` (small model, no tools, nothing saved).
#[tauri::command]
pub async fn git_message(
    cwd: String,
    account: Option<String>,
    pr: Option<bool>,
) -> Result<String, String> {
    let root = PathBuf::from(&cwd);
    let pr = pr.unwrap_or(false);
    let diffs = if pr {
        let base = default_base(&root)
            .await
            .ok_or("No base branch to compare with")?;
        git_diff(cwd.clone(), "branch".into(), Some(base)).await?
    } else {
        git_diff(cwd.clone(), "uncommitted".into(), None).await?
    };
    if diffs.is_empty() {
        return Err("There are no changes".into());
    }
    let mut text = String::new();
    for d in &diffs {
        text.push_str(&format!(
            "--- {} ({}, +{} -{})\n",
            d.path, d.kind, d.added, d.removed
        ));
        if text.len() < 60_000 {
            text.push_str(&d.diff.chars().take(8_000).collect::<String>());
            text.push('\n');
        }
    }
    let log = git(&root, &["log", "--format=%s", "-n15"])
        .await
        .unwrap_or_default();
    let ask = if pr {
        "Write a pull request for these changes. First line: a short title. Then a blank line, then a short Markdown body (what changed and why, as a few bullets). Reply with only that, no code fences."
    } else {
        "Write a git commit message for these changes in the repository's style (recent subjects below). A short Conventional Commit subject line; add a short body only if the reason is not obvious. Reply with only the message, no code fences."
    };
    let prompt = format!("{ask}\n\nRecent commit subjects:\n{log}\n\nChanges:\n{text}");
    let bin = glaude_core::env::claude_binary().ok_or("Claude Code CLI not found")?;
    let dir = account
        .as_deref()
        .and_then(glaude_accounts::paths::claude_config_dir)
        .map(|d| d.display().to_string().trim_end_matches('/').to_string());
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args([
        "-p",
        "--model",
        "haiku",
        "--no-session-persistence",
        "--tools",
        "",
        "--output-format",
        "text",
    ])
    .current_dir(&root)
    .env_clear()
    .envs(glaude_core::env::agent_env(&[("CLAUDE_CONFIG_DIR", dir)]))
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .kill_on_drop(true);
    let mut child = cmd.spawn().map_err(|e| e.to_string())?;
    if let Some(mut w) = child.stdin.take() {
        tokio::spawn(async move {
            let _ = w.write_all(prompt.as_bytes()).await;
        });
    }
    let out = tokio::time::timeout(Duration::from_secs(120), child.wait_with_output())
        .await
        .map_err(|_| "Claude took too long".to_string())?
        .map_err(|e| e.to_string())?;
    let msg = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() || msg.is_empty() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(first_line(&err)
            .or_else(|| first_line(&msg))
            .unwrap_or("Claude did not answer")
            .to_string());
    }
    Ok(strip_fences(&msg))
}

fn strip_fences(s: &str) -> String {
    let t = s.trim();
    if let Some(rest) = t.strip_prefix("```") {
        let rest = rest.split_once('\n').map(|(_, r)| r).unwrap_or("");
        return rest.trim_end().trim_end_matches("```").trim().to_string();
    }
    t.to_string()
}

/// `.glaude.json` at the repository root (or the folder itself).
#[tauri::command]
pub async fn project_config(cwd: String) -> Value {
    let cwd = PathBuf::from(cwd);
    let root = git(&cwd, &["rev-parse", "--show-toplevel"])
        .await
        .map(|r| PathBuf::from(r.trim()))
        .unwrap_or(cwd.clone());
    for dir in [&cwd, &root] {
        if let Ok(text) = std::fs::read_to_string(dir.join(".glaude.json")) {
            if let Ok(v) = serde_json::from_str::<Value>(&text) {
                return v;
            }
        }
    }
    // A worktree reads the main checkout's config when it has none.
    if let Ok(common) = git(
        &cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await
    {
        if let Some(main) = Path::new(common.trim()).parent() {
            if let Ok(text) = std::fs::read_to_string(main.join(".glaude.json")) {
                return serde_json::from_str(&text).unwrap_or(Value::Null);
            }
        }
    }
    Value::Null
}

/// A folder and branch name from a conversation title.
pub fn slug(title: &str) -> String {
    let mut s = String::new();
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            s.push(c.to_ascii_lowercase());
        } else if !s.ends_with('-') && !s.is_empty() {
            s.push('-');
        }
        if s.len() >= 40 {
            break;
        }
    }
    let s = s.trim_matches('-').to_string();
    if s.is_empty() {
        format!("chat-{}", &uuid::Uuid::new_v4().simple().to_string()[..6])
    } else {
        s
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: String,
    pub branch: String,
    /// `.glaude.json` worktree.setup, to run in a terminal there.
    pub setup: Option<String>,
}

/// A new worktree (and branch `glaude/<name>`) for a conversation.
#[tauri::command]
pub async fn worktree_add(cwd: String, name: String) -> Result<Worktree, String> {
    let cwd = PathBuf::from(&cwd);
    let root = git(&cwd, &["rev-parse", "--show-toplevel"])
        .await
        .map_err(|_| "This folder is not in a git repository".to_string())?;
    let root = PathBuf::from(root.trim());
    if git(&root, &["rev-parse", "--verify", "--quiet", "HEAD"])
        .await
        .is_err()
    {
        return Err("The repository has no commits yet".into());
    }
    let repo = root
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "repo".into());
    let base = slug(&name);
    let mut n = 1;
    let (path, branch) = loop {
        let s = if n == 1 {
            base.clone()
        } else {
            format!("{base}-{n}")
        };
        let path = worktrees_dir().join(&repo).join(&s);
        let branch = format!("glaude/{s}");
        let taken = path.exists()
            || git(
                &root,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("refs/heads/{branch}"),
                ],
            )
            .await
            .is_ok();
        if !taken {
            break (path, branch);
        }
        n += 1;
    };
    std::fs::create_dir_all(path.parent().unwrap_or(&path)).map_err(|e| e.to_string())?;
    let p = path.display().to_string();
    git(&root, &["worktree", "add", "-b", &branch, &p, "HEAD"]).await?;
    let config = project_config(root.display().to_string()).await;
    let setup = config["worktree"]["setup"].as_str().map(str::to_string);
    Ok(Worktree {
        path: p,
        branch,
        setup,
    })
}

/// Remove a worktree Glaude created (after `.glaude.json` worktree.cleanup).
#[tauri::command]
pub async fn worktree_remove(path: String, force: bool) -> Result<(), String> {
    let p = PathBuf::from(&path);
    if !is_managed(&p) {
        return Err("Only worktrees Glaude created can be removed here".into());
    }
    let config = project_config(path.clone()).await;
    if let Some(cleanup) = config["worktree"]["cleanup"].as_str() {
        let (sh, flag) = if cfg!(windows) {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };
        let _ = run(sh, &p, &[flag, cleanup], None, 120).await;
    }
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path);
    // Run from the main checkout: the worktree itself is going away.
    let common = git(
        &p,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .await?;
    let main = Path::new(common.trim())
        .parent()
        .map(Path::to_path_buf)
        .ok_or("No main checkout")?;
    git(&main, &args).await.map_err(|e| {
        if e.contains("modified or untracked") {
            "The worktree has uncommitted changes".to_string()
        } else {
            e
        }
    })?;
    let _ = git(&main, &["worktree", "prune"]).await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_git_diff_by_file() {
        let text = "diff --git a/src/a.rs b/src/a.rs\nindex 1..2 100644\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,2 @@\n-old\n+new\n ctx\ndiff --git a/new.txt b/new.txt\nnew file mode 100644\nindex 0..3\n--- /dev/null\n+++ b/new.txt\n@@ -0,0 +1 @@\n+hello\ndiff --git a/gone.md b/gone.md\ndeleted file mode 100644\n--- a/gone.md\n+++ /dev/null\n@@ -1 +0,0 @@\n-bye\ndiff --git a/img.png b/img.png\nBinary files a/img.png and b/img.png differ\n";
        let d = parse_diff(text);
        assert_eq!(d.len(), 4);
        assert_eq!(
            (
                d[0].path.as_str(),
                d[0].kind.as_str(),
                d[0].added,
                d[0].removed
            ),
            ("src/a.rs", "update", 1, 1)
        );
        assert!(d[0].diff.starts_with("@@ -1,2 +1,2 @@"));
        assert_eq!((d[1].path.as_str(), d[1].kind.as_str()), ("new.txt", "add"));
        assert_eq!(
            (d[2].path.as_str(), d[2].kind.as_str()),
            ("gone.md", "delete")
        );
        assert!(d[3].diff.contains("Binary"));
    }

    #[test]
    fn slugs_titles() {
        assert_eq!(slug("Fix checkout rounding!"), "fix-checkout-rounding");
        assert!(slug("رفع مشکل").starts_with("chat-"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn diff_commit_and_worktree_in_a_temp_repo() {
        let dir = std::env::temp_dir().join(format!("glaude-git-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        // Worktrees go under the data folder: keep them in the temp folder.
        let data = std::env::temp_dir().join(format!("glaude-git-data-{}", std::process::id()));
        // SAFETY: only this test reads GLAUDE_HOST_DIR in the lib tests.
        unsafe { std::env::set_var("GLAUDE_HOST_DIR", &data) };
        let d = dir.display().to_string();
        for args in [
            vec!["init", "-q", "-b", "main"],
            vec!["config", "user.email", "t@example.com"],
            vec!["config", "user.name", "T"],
            vec!["config", "commit.gpgsign", "false"],
        ] {
            git(&dir, &args).await.unwrap();
        }
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        let info = git_info(d.clone()).await.unwrap();
        assert!(!info.has_commits);
        assert_eq!(info.dirty, 1);
        let changes = git_diff(d.clone(), "uncommitted".into(), None)
            .await
            .unwrap();
        assert_eq!(changes.len(), 1, "{changes:?}");
        assert_eq!(changes[0].kind, "add");
        let sha = git_commit(d.clone(), "feat: first".into()).await.unwrap();
        assert!(!sha.is_empty());
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        std::fs::write(dir.join("b.txt"), "new\n").unwrap();
        let changes = git_diff(d.clone(), "uncommitted".into(), None)
            .await
            .unwrap();
        assert_eq!(changes.len(), 2, "{changes:?}");
        let log = git_log(d.clone(), None, None).await.unwrap();
        assert_eq!(log[0].subject, "feat: first");
        let one = git_diff(d.clone(), "commit".into(), Some(log[0].sha.clone()))
            .await
            .unwrap();
        assert_eq!(one[0].path, "a.txt");

        let wt = worktree_add(d.clone(), "Try a thing".into()).await.unwrap();
        assert_eq!(wt.branch, "glaude/try-a-thing");
        assert!(Path::new(&wt.path).join("a.txt").exists());
        let winfo = git_info(wt.path.clone()).await.unwrap();
        assert!(winfo.worktree && winfo.managed, "{winfo:?}");
        worktree_remove(wt.path.clone(), true).await.unwrap();
        assert!(!Path::new(&wt.path).exists());
        let _ = git(&dir, &["branch", "-D", &wt.branch]).await;
        std::fs::remove_dir_all(&dir).ok();
        std::fs::remove_dir_all(&data).ok();
    }
}
