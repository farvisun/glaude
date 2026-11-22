//! Unified diffs for the diff viewer.

use similar::{ChangeTag, TextDiff};

use crate::model::FileDiff;

/// Build a unified diff between two versions of a file.
pub fn unified(path: &str, old: &str, new: &str) -> FileDiff {
    let diff = TextDiff::from_lines(old, new);
    let (mut added, mut removed) = (0u32, 0u32);
    for change in diff.iter_all_changes() {
        match change.tag() {
            ChangeTag::Insert => added += 1,
            ChangeTag::Delete => removed += 1,
            ChangeTag::Equal => {}
        }
    }
    let text = diff
        .unified_diff()
        .context_radius(3)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string();
    let kind = if old.is_empty() && !new.is_empty() {
        "add"
    } else if new.is_empty() && !old.is_empty() {
        "delete"
    } else {
        "update"
    };
    FileDiff {
        path: path.to_string(),
        kind: kind.to_string(),
        diff: text,
        added,
        removed,
    }
}

/// Count +/- lines of an existing unified diff.
pub fn count(diff: &str) -> (u32, u32) {
    let (mut a, mut r) = (0u32, 0u32);
    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            continue;
        }
        if line.starts_with('+') {
            a += 1;
        } else if line.starts_with('-') {
            r += 1;
        }
    }
    (a, r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_changes() {
        let d = unified("x.txt", "a\nb\nc\n", "a\nB\nc\nd\n");
        assert_eq!((d.added, d.removed), (2, 1));
        assert_eq!(d.kind, "update");
        assert!(d.diff.contains("@@"));
        assert_eq!(count(&d.diff), (2, 1));
    }

    #[test]
    fn new_file_is_add() {
        assert_eq!(unified("n.txt", "", "hi\n").kind, "add");
    }
}
