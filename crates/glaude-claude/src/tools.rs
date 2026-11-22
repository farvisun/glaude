//! Tool display helpers: one-line titles and diff previews.

use std::path::Path;

use glaude_core::diff;
use glaude_core::model::FileDiff;
use serde_json::Value;

const MAX_PREVIEW_FILE: u64 = 2_000_000;

fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

/// Show paths relative to the conversation folder when possible.
pub fn rel(path: &str, cwd: &str) -> String {
    let cwd = cwd.trim_end_matches('/');
    if !cwd.is_empty() {
        if let Some(rest) = path.strip_prefix(cwd) {
            if let Some(r) = rest.strip_prefix('/') {
                return r.to_string();
            }
        }
    }
    path.to_string()
}

fn first_line(text: &str, max: usize) -> String {
    let line = text.lines().next().unwrap_or("").trim();
    if line.chars().count() > max {
        let cut: String = line.chars().take(max).collect();
        format!("{cut}…")
    } else if text.lines().nth(1).is_some() {
        format!("{line} …")
    } else {
        line.to_string()
    }
}

/// A short one-line description of a tool call.
pub fn title(name: &str, input: &Value, cwd: &str) -> String {
    let path = || {
        s(input, "file_path")
            .or(s(input, "notebook_path"))
            .map(|p| rel(p, cwd))
    };
    match name {
        "Bash" => s(input, "command")
            .map(|c| first_line(c, 200))
            .unwrap_or_default(),
        "Read" | "Write" | "Edit" | "MultiEdit" | "NotebookEdit" | "NotebookRead" => {
            path().unwrap_or_default()
        }
        "Grep" => {
            let pat = s(input, "pattern").unwrap_or("");
            match s(input, "path") {
                Some(p) => format!("{pat}  {}", rel(p, cwd)),
                None => pat.to_string(),
            }
        }
        "Glob" => s(input, "pattern").unwrap_or("").to_string(),
        "WebFetch" => s(input, "url").unwrap_or("").to_string(),
        "WebSearch" => s(input, "query").unwrap_or("").to_string(),
        "Task" | "Agent" => s(input, "description")
            .or(s(input, "prompt"))
            .map(|d| first_line(d, 120))
            .unwrap_or_default(),
        "TodoWrite" => {
            let n = input
                .get("todos")
                .and_then(Value::as_array)
                .map_or(0, Vec::len);
            format!("{n} todos")
        }
        "Skill" => s(input, "skill")
            .or(s(input, "command"))
            .unwrap_or("")
            .to_string(),
        _ => input
            .as_object()
            .and_then(|o| o.values().find_map(Value::as_str))
            .map(|v| first_line(v, 120))
            .unwrap_or_default(),
    }
}

fn read_small(path: &str) -> Option<String> {
    let p = Path::new(path);
    let meta = p.metadata().ok()?;
    if meta.len() > MAX_PREVIEW_FILE {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

fn apply_edit(text: &str, old: &str, new: &str, all: bool) -> Option<String> {
    if old.is_empty() || !text.contains(old) {
        return None;
    }
    Some(if all {
        text.replace(old, new)
    } else {
        text.replacen(old, new, 1)
    })
}

/// Diff preview for file-editing tools, computed against the file on disk
/// so line numbers are real. Falls back to old/new strings.
pub fn diff_preview(name: &str, input: &Value, cwd: &str) -> Vec<FileDiff> {
    let Some(path) = s(input, "file_path") else {
        return Vec::new();
    };
    let shown = rel(path, cwd);
    match name {
        "Write" => {
            let new = s(input, "content").unwrap_or("");
            let old = read_small(path).unwrap_or_default();
            vec![diff::unified(&shown, &old, new)]
        }
        "Edit" => {
            let old_s = s(input, "old_string").unwrap_or("");
            let new_s = s(input, "new_string").unwrap_or("");
            let all = input
                .get("replace_all")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            match read_small(path).and_then(|t| apply_edit(&t, old_s, new_s, all).map(|n| (t, n))) {
                Some((t, n)) => vec![diff::unified(&shown, &t, &n)],
                None => vec![diff::unified(&shown, old_s, new_s)],
            }
        }
        "MultiEdit" => {
            let edits = input
                .get("edits")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if let Some(orig) = read_small(path) {
                let mut cur = orig.clone();
                let mut ok = true;
                for e in &edits {
                    let all = e
                        .get("replace_all")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    match apply_edit(
                        &cur,
                        s(e, "old_string").unwrap_or(""),
                        s(e, "new_string").unwrap_or(""),
                        all,
                    ) {
                        Some(n) => cur = n,
                        None => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    return vec![diff::unified(&shown, &orig, &cur)];
                }
            }
            let old: Vec<&str> = edits.iter().filter_map(|e| s(e, "old_string")).collect();
            let new: Vec<&str> = edits.iter().filter_map(|e| s(e, "new_string")).collect();
            vec![diff::unified(&shown, &old.join("\n"), &new.join("\n"))]
        }
        _ => Vec::new(),
    }
}

/// Tool result content as plain text.
pub fn result_text(content: &Value) -> String {
    match content {
        Value::String(s) => s.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|p| match p.get("type").and_then(Value::as_str) {
                Some("text") => p.get("text").and_then(Value::as_str).map(str::to_string),
                Some("image") => Some("[image]".to_string()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The questions of an AskUserQuestion call.
pub fn questions(input: &Value) -> Vec<glaude_core::model::Question> {
    use glaude_core::model::{Question, QuestionOption};
    let str_of = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    input
        .get("questions")
        .and_then(Value::as_array)
        .map(|qs| {
            qs.iter()
                .filter_map(|q| {
                    Some(Question {
                        question: str_of(q, "question")?,
                        header: str_of(q, "header").unwrap_or_default(),
                        multi_select: q
                            .get("multiSelect")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                        options: q
                            .get("options")
                            .and_then(Value::as_array)
                            .map(|os| {
                                os.iter()
                                    .filter_map(|o| {
                                        Some(QuestionOption {
                                            label: str_of(o, "label")?,
                                            description: str_of(o, "description"),
                                            preview: str_of(o, "preview"),
                                        })
                                    })
                                    .collect()
                            })
                            .unwrap_or_default(),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn titles() {
        assert_eq!(
            title("Bash", &json!({"command": "ls -la\npwd"}), "/p"),
            "ls -la …"
        );
        assert_eq!(
            title("Read", &json!({"file_path": "/p/src/a.rs"}), "/p"),
            "src/a.rs"
        );
        assert_eq!(
            title("TodoWrite", &json!({"todos": [1, 2]}), "/p"),
            "2 todos"
        );
        assert_eq!(
            title("Grep", &json!({"pattern": "fn", "path": "/p/src"}), "/p"),
            "fn  src"
        );
    }

    #[test]
    fn edit_preview_falls_back_without_file() {
        let d = diff_preview(
            "Edit",
            &json!({"file_path": "/nonexistent/x.rs", "old_string": "a", "new_string": "b"}),
            "/",
        );
        assert_eq!(d.len(), 1);
        assert_eq!((d[0].added, d[0].removed), (1, 1));
    }

    #[test]
    fn result_text_joins_parts() {
        assert_eq!(
            result_text(&json!([{"type": "text", "text": "a"}, {"type": "text", "text": "b"}])),
            "a\nb"
        );
    }
}
