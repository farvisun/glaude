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
