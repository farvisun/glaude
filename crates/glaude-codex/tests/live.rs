//! Live test against the installed `codex` CLI (default account, smallest
//! model). Uses a little quota, so it only runs on request:
//!
//!     cargo test -p glaude-codex --test live -- --ignored --nocapture

use std::sync::Arc;
use std::time::Duration;

use glaude_codex::{CodexServer, CodexSpec, CodexThread};
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

fn sink(log: &Log) -> glaude_codex::Sink {
    let log = log.clone();
    Arc::new(move |evs: Vec<ConvEvent>| log.lock().extend(evs))
}

fn model() -> String {
    std::env::var("GLAUDE_CODEX_TEST_MODEL").unwrap_or_else(|_| "gpt-6-luna".into())
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn command_with_approval_then_resume() {
    let dir = std::env::temp_dir().join(format!("glaude-codex-live-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("note.txt"), "hello fixture\n").unwrap();
    let cwd = dir.canonicalize().unwrap().display().to_string();
    let home = dirs::home_dir().unwrap().join(".codex");

    let server = CodexServer::start(&home, None).await.expect("app-server");
    let log: Log = Arc::default();
    let spec = CodexSpec {
        cwd: cwd.clone(),
        model: Some(model()),
        effort: Some("low".into()),
        mode: Some(std::env::var("GLAUDE_CODEX_TEST_MODE").unwrap_or_else(|_| "untrusted".into())),
        ..Default::default()
    };
    let t = CodexThread::open(server.clone(), spec.clone(), sink(&log))
        .await
        .expect("thread");
    assert!(
        log.lock()
            .iter()
            .any(|e| matches!(e, ConvEvent::Session { info } if !info.models.is_empty())),
        "models listed"
    );
    t.send(
        "Run exactly this shell command: printf 'سلام' > out.txt && cat note.txt . Then reply in Persian in one short sentence.",
        vec![],
    )
    .await
    .expect("turn/start");

    let perm = wait_for(&log, 150, |e| {
        matches!(
            e,
            ConvEvent::Item {
                item: Item::Permission {
                    status: Status::Pending,
                    ..
                }
            }
        ) || matches!(e, ConvEvent::TurnDone { .. })
    })
    .await
    .expect("approval or turn end");
    if let ConvEvent::Item {
        item: Item::Permission {
            request_id, title, ..
        },
    } = perm
    {
        eprintln!("approval: {title}");
        t.answer(&request_id, "allow");
    } else {
        eprintln!("no approval was requested");
    }
    let done = wait_for(&log, 150, |e| matches!(e, ConvEvent::TurnDone { .. }))
        .await
        .expect("turn done");
    eprintln!("{done:?}");
    let cmd = log.lock().iter().rev().find_map(|e| match e {
        ConvEvent::Item {
            item: Item::Command { status, output, .. },
        } if *status != Status::Running => Some((*status, output.clone())),
        _ => None,
    });
    eprintln!("command: {cmd:?}");
    assert_eq!(
        std::fs::read_to_string(dir.join("out.txt")).unwrap_or_default(),
        "سلام"
    );
    let (status, output) = cmd.expect("command item");
    assert_eq!(status, Status::Done);
    assert!(output.contains("hello fixture"));
    let thread_id = t.thread_id.clone();
    t.close();

    // Resume on the same server: history comes back first.
    let log2: Log = Arc::default();
    let t2 = CodexThread::open(
        server.clone(),
        CodexSpec {
            resume: Some(thread_id),
            load_history: true,
            ..spec
        },
        sink(&log2),
    )
    .await
    .expect("resume");
    let items = log2
        .lock()
        .iter()
        .find_map(|e| match e {
            ConvEvent::Reset { items } => Some(items.clone()),
            _ => None,
        })
        .expect("history");
    assert!(
        items.iter().any(|i| matches!(i, Item::User { .. })),
        "{items:?}"
    );
    assert!(
        items.iter().any(|i| matches!(i, Item::Command { .. })),
        "{items:?}"
    );
    assert!(
        items.iter().any(|i| matches!(i, Item::Assistant { .. })),
        "{items:?}"
    );
    t2.close();
    server.shutdown();
    std::fs::remove_dir_all(dir).ok();
}

#[tokio::test(flavor = "multi_thread")]
#[ignore]
async fn lists_threads_and_models() {
    let home = dirs::home_dir().unwrap().join(".codex");
    let server = CodexServer::start(&home, None).await.expect("app-server");
    let threads = server.list_threads(150).await.expect("thread/list");
    eprintln!(
        "{} threads; newest: {:?}",
        threads.len(),
        threads.first().map(|t| (&t.title, t.updated))
    );
    assert!(
        threads.windows(2).all(|w| w[0].updated >= w[1].updated),
        "newest first"
    );
    assert!(!server.models().await.expect("models").is_empty());
    server.shutdown();
}
