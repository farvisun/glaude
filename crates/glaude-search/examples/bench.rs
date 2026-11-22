//! Index real history into a scratch database and time a few queries.
//! Prints counts and timings only.
//!
//!     cargo run --release -p glaude-search --example bench -- <db> <claude-projects> <codex-sessions> [query…]

use std::time::Instant;

use glaude_search::{Index, Kind, Source};

fn main() -> anyhow::Result<()> {
    let a: Vec<String> = std::env::args().collect();
    let idx = Index::open(a[1].as_ref())?;
    let sources = [
        Source {
            kind: Kind::Claude,
            root: a[2].clone().into(),
        },
        Source {
            kind: Kind::Codex,
            root: a[3].clone().into(),
        },
    ];
    let t = Instant::now();
    let st = idx.update(&sources, &mut |_| {})?;
    println!(
        "update: {:?} in {:.1}s, total messages {}",
        st,
        t.elapsed().as_secs_f32(),
        idx.message_count()
    );
    for q in &a[4..] {
        let t = Instant::now();
        let hits = idx.search(q, 50)?;
        println!(
            "query #{}: {} sessions, {} ms",
            q.chars().count(),
            hits.len(),
            t.elapsed().as_millis()
        );
    }
    Ok(())
}
