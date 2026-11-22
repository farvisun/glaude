//! Current git branch of a folder, read from `.git/HEAD` (no git process).

use std::fs;
use std::path::{Path, PathBuf};

/// The `.git` directory for `start` or a parent, following worktree
/// `gitdir:` files.
fn git_dir(start: &Path) -> Option<PathBuf> {
    let mut dir = Some(start);
    while let Some(d) = dir {
        let dot = d.join(".git");
        if dot.is_dir() {
            return Some(dot);
        }
        if dot.is_file() {
            let text = fs::read_to_string(&dot).ok()?;
            let target = text.trim().strip_prefix("gitdir:")?.trim();
            let p = PathBuf::from(target);
            return Some(if p.is_absolute() { p } else { d.join(p) });
        }
        dir = d.parent();
    }
    None
}

/// Branch name, or a short commit id when HEAD is detached.
pub fn branch(cwd: &Path) -> Option<String> {
    let head = fs::read_to_string(git_dir(cwd)?.join("HEAD")).ok()?;
    let head = head.trim();
    if let Some(r) = head.strip_prefix("ref:") {
        let r = r.trim();
        return Some(r.strip_prefix("refs/heads/").unwrap_or(r).to_string());
    }
    (head.len() >= 7).then(|| head[..7].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_branch_and_worktree() {
        let root = std::env::temp_dir().join(format!("glaude-git-{}", std::process::id()));
        let sub = root.join("a/b");
        fs::create_dir_all(&sub).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join(".git/HEAD"), "ref: refs/heads/feature/x\n").unwrap();
        assert_eq!(branch(&sub).as_deref(), Some("feature/x"));

        let wt = root.join("wt");
        fs::create_dir_all(root.join(".git/worktrees/wt")).unwrap();
        fs::write(root.join(".git/worktrees/wt/HEAD"), "0123456789abcdef\n").unwrap();
        fs::create_dir_all(&wt).unwrap();
        fs::write(
            wt.join(".git"),
            format!("gitdir: {}\n", root.join(".git/worktrees/wt").display()),
        )
        .unwrap();
        assert_eq!(branch(&wt).as_deref(), Some("0123456"));
        fs::remove_dir_all(root).ok();
    }
}
