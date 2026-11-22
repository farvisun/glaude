//! Batches conversation events and ships them to the webview over a
//! Channel at most every FLUSH_MS, merging streaming block updates so a
//! fast stream costs one IPC message per frame, not one per token.

use std::sync::Arc;
use std::time::Duration;

use glaude_core::model::ConvEvent;
use parking_lot::Mutex;
use tauri::ipc::Channel;

const FLUSH_MS: u64 = 24;

pub struct EventSink {
    queue: Mutex<Vec<ConvEvent>>,
    closed: Mutex<bool>,
}

/// Merge `next` into the queue, collapsing consecutive block updates of the
/// same item.
fn push_merged(queue: &mut Vec<ConvEvent>, next: ConvEvent) {
    if let ConvEvent::Blocks { id, from, blocks } = &next {
        if let Some(ConvEvent::Blocks {
            id: pid,
            from: pfrom,
            blocks: pblocks,
        }) = queue.last_mut()
        {
            if pid == id {
                if *from <= *pfrom {
                    *pfrom = *from;
                    *pblocks = blocks.clone();
                } else {
                    pblocks.truncate(from - *pfrom);
                    pblocks.extend(blocks.iter().cloned());
                }
                return;
            }
        }
    }
    queue.push(next);
}

impl EventSink {
    pub fn new(channel: Channel<Vec<ConvEvent>>) -> Arc<Self> {
        let sink = Arc::new(Self {
            queue: Mutex::new(Vec::new()),
            closed: Mutex::new(false),
        });
        let weak = Arc::downgrade(&sink);
        tauri::async_runtime::spawn(async move {
            let mut tick = tokio::time::interval(Duration::from_millis(FLUSH_MS));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                let Some(s) = weak.upgrade() else { break };
                let batch = std::mem::take(&mut *s.queue.lock());
                if !batch.is_empty() && channel.send(batch).is_err() {
                    break;
                }
                if *s.closed.lock() && s.queue.lock().is_empty() {
                    break;
                }
            }
        });
        sink
    }

    pub fn push(&self, events: Vec<ConvEvent>) {
        let mut q = self.queue.lock();
        for e in events {
            push_merged(&mut q, e);
        }
    }

    pub fn close(&self) {
        *self.closed.lock() = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glaude_core::bidi::Dir;
    use glaude_core::model::Block;

    fn b(t: &str) -> Block {
        Block {
            html: t.into(),
            dir: Dir::Ltr,
        }
    }

    #[test]
    fn merges_tail_updates() {
        let mut q = Vec::new();
        push_merged(
            &mut q,
            ConvEvent::Blocks {
                id: "a".into(),
                from: 0,
                blocks: vec![b("1")],
            },
        );
        push_merged(
            &mut q,
            ConvEvent::Blocks {
                id: "a".into(),
                from: 0,
                blocks: vec![b("12")],
            },
        );
        push_merged(
            &mut q,
            ConvEvent::Blocks {
                id: "a".into(),
                from: 1,
                blocks: vec![b("x")],
            },
        );
        assert_eq!(q.len(), 1);
        match &q[0] {
            ConvEvent::Blocks { from, blocks, .. } => {
                assert_eq!(*from, 0);
                assert_eq!(
                    blocks.iter().map(|b| b.html.as_str()).collect::<Vec<_>>(),
                    ["12", "x"]
                );
            }
            _ => unreachable!(),
        }
    }
}
