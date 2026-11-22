//! Live test against the installed `claude` CLI. Costs a few cents of the
//! default account's quota, so it only runs on request:
//!
//!     cargo test -p glaude-claude --test live -- --ignored --nocapture

use std::sync::Arc;
use std::time::Duration;

use glaude_claude::{ClaudeSession, ClaudeSpec};
use glaude_core::model::{ConvEvent, Item, Status};
use parking_lot::Mutex;

type Log = Arc<Mutex<Vec<ConvEvent>>>;

async fn wait_for(log: &Log, secs: u64, pred: impl Fn(&ConvEvent) -> bool) -> Option<ConvEvent> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(secs);
    let mut seen = 0;
    while tokio::time::Instant::now() < deadline {
        {
            let l = log.lock();
            for e in &l[seen..] {
                if pred(e) {
                    return Some(e.clone());
                }
            }
            seen = l.len();
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    None
}

fn sink(log: &Log) -> glaude_claude::Sink {
    let log = log.clone();
    Arc::new(move |evs: Vec<ConvEvent>| log.lock().extend(evs))
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn write_with_permission_then_resume() {
    let dir = std::env::temp_dir().join(format!("glaude-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let cwd = dir.canonicalize().unwrap().display().to_string();

    let log: Log = Arc::default();
    let spec = ClaudeSpec {
        cwd: cwd.clone(),
        model: Some("haiku".into()),
        permission_mode: Some("default".into()),
        ..Default::default()
    };
    let s = ClaudeSession::spawn(spec.clone(), sink(&log))
        .await
        .expect("spawn");
    s.send("Use the Write tool to create hi.txt containing exactly: سلام. Then reply with one Persian word.", vec![], vec![]);

    let perm = wait_for(&log, 90, |e| {
        matches!(
            e,
            ConvEvent::Item {
                item: Item::Permission {
                    status: Status::Pending,
                    ..
                }
            }
        )
    })
    .await
    .expect("permission prompt");
    let ConvEvent::Item {
        item: Item::Permission {
            request_id, diffs, ..
        },
    } = perm
    else {
        unreachable!()
    };
    assert_eq!(diffs.len(), 1, "write shows a diff");
    s.answer(&request_id, "allow", None);

    wait_for(&log, 90, |e| matches!(e, ConvEvent::TurnDone { .. }))
        .await
        .expect("turn done");
    assert_eq!(
        std::fs::read_to_string(dir.join("hi.txt")).unwrap().trim(),
        "سلام"
    );
    let session_id = log
        .lock()
        .iter()
        .find_map(|e| match e {
            ConvEvent::Session { info } => info.session_id.clone(),
            _ => None,
        })
        .expect("session id");
    // Usage/context arrive right after the turn.
    wait_for(
        &log,
        20,
        |e| matches!(e, ConvEvent::Session { info } if info.context_window.is_some()),
    )
    .await
    .expect("context usage");

    s.close();
    wait_for(&log, 10, |e| matches!(e, ConvEvent::Exited { .. }))
        .await
        .expect("exited");
    drop(s);

    // Resume: the transcript is replayed first.
    let log2: Log = Arc::default();
    let s2 = ClaudeSession::spawn(
        ClaudeSpec {
            resume: Some(session_id),
            load_history: true,
            ..spec
        },
        sink(&log2),
    )
    .await
    .expect("resume");
    let reset = log2.lock().first().cloned().expect("history first");
    match reset {
        ConvEvent::Reset { items } => {
            assert!(
                items.iter().any(|i| matches!(i, Item::User { .. })),
                "{items:?}"
            );
            assert!(items
                .iter()
                .any(|i| matches!(i, Item::Tool { name, .. } if name == "Write")));
        }
        other => panic!("expected reset, got {other:?}"),
    }
    s2.close();
    std::fs::remove_dir_all(dir).ok();
}

fn last_turn_done_after(log: &Log, from: usize) -> bool {
    log.lock()[from..]
        .iter()
        .any(|e| matches!(e, ConvEvent::TurnDone { .. }))
}

/// Control requests, AskUserQuestion, a background subagent and file rewind.
///
///     cargo test -p glaude-claude --test live agents -- --ignored --nocapture
#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn agents_questions_and_rewind() {
    let dir = std::env::temp_dir().join(format!("glaude-live-agents-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("notes.txt"), "hello\n").unwrap();
    let cwd = dir.canonicalize().unwrap().display().to_string();

    let log: Log = Arc::default();
    let s = ClaudeSession::spawn(
        ClaudeSpec {
            cwd: cwd.clone(),
            model: Some("haiku".into()),
            permission_mode: Some("acceptEdits".into()),
            ..Default::default()
        },
        sink(&log),
    )
    .await
    .expect("spawn");

    // 1. Control requests answered to the caller, not the transcript.
    let ctx = s
        .request(serde_json::json!({"subtype": "get_context_usage"}))
        .await
        .expect("context usage");
    assert!(ctx["maxTokens"].as_u64().unwrap_or(0) > 0, "{ctx}");
    // The file index warms up after start; an empty query lists the folder.
    let mut files = serde_json::Value::Null;
    for query in ["", "no", "no", "no", "no", "no"] {
        files = s
            .request(serde_json::json!({"subtype": "file_suggestions", "query": query}))
            .await
            .expect("file suggestions");
        if query == "no" && files.to_string().contains("notes.txt") {
            break;
        }
        tokio::time::sleep(Duration::from_millis(700)).await;
    }
    println!("file suggestions for 'no': {files}");
    assert!(files.to_string().contains("notes.txt"), "{files}");
    let bad = s
        .request(serde_json::json!({"subtype": "get_task_output", "task_id": "nope"}))
        .await;
    assert!(bad.is_err(), "unknown task is an error");
    let agents = log.lock().iter().any(|e| {
        matches!(e, ConvEvent::Session { info } if info.agents.iter().any(|x| x.name == "Explore"))
    });
    assert!(agents, "initialize lists the agents");

    // 2. AskUserQuestion becomes a form; the answer reaches the model.
    s.send(
        "Use the AskUserQuestion tool to ask me which color I prefer, with the options Red and Blue. Then reply with only the color I chose.",
        vec![],
        vec![],
    );
    let q = wait_for(&log, 120, |e| {
        matches!(
            e,
            ConvEvent::Item {
                item: Item::Question {
                    status: Status::Pending,
                    ..
                }
            }
        )
    })
    .await
    .expect("question");
    let ConvEvent::Item {
        item:
            Item::Question {
                request_id,
                questions,
                ..
            },
    } = q
    else {
        unreachable!()
    };
    let mut answers = std::collections::BTreeMap::new();
    answers.insert(questions[0].question.clone(), "Blue".to_string());
    let mark = log.lock().len();
    s.answer_question(&request_id, answers);
    wait_for(&log, 120, |e| matches!(e, ConvEvent::TurnDone { .. }))
        .await
        .expect("turn after answer");
    let replied = log.lock()[mark..].iter().any(|e| {
        matches!(e, ConvEvent::Item { item: Item::Assistant { blocks, streaming: false, .. } } if blocks.iter().any(|b| b.html.contains("Blue")))
    });
    assert!(replied, "the model saw the answer");

    // 3. A background subagent: task events, then a report.
    let mark = log.lock().len();
    s.send(
        "Use the Agent tool with subagent_type Explore to list the files in the current folder. When it reports back, reply with just: done",
        vec![],
        vec![],
    );
    let finished = wait_for(&log, 240, |e| {
        matches!(e, ConvEvent::Task { task } if task.kind == "agent" && task.status != "running" && task.status != "pending")
    })
    .await
    .expect("agent task finished");
    let ConvEvent::Task { task } = finished else {
        unreachable!()
    };
    println!(
        "agent task: {} {} report={:?}",
        task.status,
        task.description,
        task.report
            .as_deref()
            .map(|r| r.chars().take(80).collect::<String>())
    );
    assert_eq!(task.status, "completed");
    assert!(task.tool_use_id.is_some());
    let started = log.lock()[mark..]
        .iter()
        .any(|e| matches!(e, ConvEvent::Task { task } if task.status == "running"));
    assert!(started, "a running task event came first");
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    let after = log.lock().len();
    while !last_turn_done_after(&log, after) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }

    let report = log.lock().iter().rev().find_map(|e| match e {
        ConvEvent::Task { task } if task.kind == "agent" => task.report.clone(),
        _ => None,
    });
    println!(
        "agent report: {:?}",
        report
            .as_deref()
            .map(|r| r.chars().take(100).collect::<String>())
    );
    assert!(
        report.is_some_and(|r| r.contains("notes.txt")),
        "the report names the file"
    );

    // 4. Rewind file edits made since a message.
    s.send(
        "Use the Write tool to create made.txt containing x. Reply with ok.",
        vec![],
        vec![],
    );
    let user_id = wait_for(&log, 30, |e| {
        matches!(e, ConvEvent::Item { item: Item::User { blocks, .. } } if blocks.iter().any(|b| b.html.contains("made.txt")))
    })
    .await
    .and_then(|e| match e {
        ConvEvent::Item { item } => Some(item.id().to_string()),
        _ => None,
    })
    .expect("user message");
    let after = log.lock().len();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(120);
    while !last_turn_done_after(&log, after) && tokio::time::Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert!(dir.join("made.txt").exists(), "file written");
    let dry = s
        .request(serde_json::json!({"subtype": "rewind_files", "user_message_id": user_id, "dry_run": true}))
        .await
        .expect("dry run");
    println!("rewind dry run: {dry}");
    assert_eq!(dry["canRewind"], true, "{dry}");
    assert!(
        dry["filesChanged"].to_string().contains("made.txt"),
        "{dry}"
    );
    s.request(serde_json::json!({"subtype": "rewind_files", "user_message_id": user_id}))
        .await
        .expect("rewind");
    assert!(
        !dir.join("made.txt").exists(),
        "rewind removed the new file"
    );

    // 5. The conversation rewind point is the message before it.
    let sid = log
        .lock()
        .iter()
        .rev()
        .find_map(|e| match e {
            ConvEvent::Session { info } => info.session_id.clone(),
            _ => None,
        })
        .expect("session id");
    let home = glaude_core::env::home_dir().join(".claude");
    let path = glaude_claude::transcript::find_transcript(&home, &cwd, &sid).expect("transcript");
    let parent =
        glaude_claude::transcript::parent_of(&path, &user_id).expect("user message in transcript");
    assert!(parent.is_some(), "a later message has a parent");

    s.close();
    std::fs::remove_dir_all(dir).ok();
}
