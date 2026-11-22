//! What the host remembers of a conversation, so a client that attaches
//! later (the app was closed or reloaded) sees the same thing it would have
//! seen live: the host applies every event the way the UI store does.

use std::collections::{BTreeMap, HashMap};

use glaude_core::model::{
    ConvEvent, Item, Limits, QueuedMessage, SessionInfo, Status, TaskInfo, TurnState,
};

#[derive(Default)]
pub struct Snapshot {
    items: Vec<Item>,
    index: HashMap<String, usize>,
    pub session: SessionInfo,
    limits: Limits,
    tasks: BTreeMap<String, TaskInfo>,
    pub turn: Option<TurnState>,
    turn_detail: Option<String>,
    pub queue: Vec<QueuedMessage>,
    pub limited_until: Option<String>,
    pub resume_at: Option<String>,
    /// When a permission or question started waiting (ms), if one is.
    pub waiting_since: Option<u64>,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Snapshot {
    fn upsert(&mut self, item: Item) {
        let id = item.id().to_string();
        match self.index.get(&id) {
            Some(&at) => self.items[at] = item,
            None => {
                self.index.insert(id, self.items.len());
                self.items.push(item);
            }
        }
    }

    fn reindex(&mut self) {
        self.index = self
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| (it.id().to_string(), i))
            .collect();
    }

    pub fn apply(&mut self, ev: &ConvEvent) {
        match ev {
            ConvEvent::Item { item } => self.upsert(item.clone()),
            ConvEvent::Blocks { id, from, blocks } => {
                let Some(&at) = self.index.get(id) else {
                    return;
                };
                if let Item::Assistant { blocks: b, .. }
                | Item::Reasoning { blocks: b, .. }
                | Item::User { blocks: b, .. } = &mut self.items[at]
                {
                    b.truncate(*from);
                    b.extend(blocks.iter().cloned());
                }
            }
            ConvEvent::Output { id, chunk } => {
                let Some(&at) = self.index.get(id) else {
                    return;
                };
                match &mut self.items[at] {
                    Item::Command { output, .. } => output.push_str(chunk),
                    Item::Tool { output, .. } => output.get_or_insert_default().push_str(chunk),
                    _ => {}
                }
            }
            ConvEvent::Turn { state, detail } => {
                self.turn = Some(*state);
                self.turn_detail.clone_from(detail);
            }
            ConvEvent::TurnDone { .. } => {
                self.turn = Some(TurnState::Idle);
                self.turn_detail = None;
            }
            ConvEvent::Session { info } => self.session.merge(info.clone()),
            ConvEvent::Limits { limits } => self.limits.merge(limits.clone()),
            ConvEvent::Task { task } => {
                self.tasks.insert(task.id.clone(), task.clone());
            }
            ConvEvent::Reset { items } => {
                self.items = items.clone();
                self.reindex();
                if items.is_empty() {
                    self.tasks.clear();
                }
            }
            ConvEvent::Queue { items } => self.queue = items.clone(),
            ConvEvent::Schedule {
                limited_until,
                resume_at,
            } => {
                self.limited_until.clone_from(limited_until);
                self.resume_at.clone_from(resume_at);
            }
            ConvEvent::Exited { .. } => {
                self.turn = Some(TurnState::Idle);
                for t in self.tasks.values_mut() {
                    if t.status == "running" || t.status == "pending" {
                        t.status = "stopped".into();
                    }
                }
            }
            ConvEvent::Tokens { .. } | ConvEvent::Notice { .. } | ConvEvent::Limited { .. } => {}
        }
        let waiting = self.pending_prompt();
        match (waiting, self.waiting_since) {
            (true, None) => self.waiting_since = Some(now_ms()),
            (false, Some(_)) => self.waiting_since = None,
            _ => {}
        }
    }

    /// Reset time of a used-up usage window, if any.
    pub fn limits_reset(&self) -> Option<String> {
        self.limits.reset_of_full()
    }

    pub fn pending_prompt(&self) -> bool {
        self.items.iter().rev().take(50).any(|i| {
            matches!(
                i,
                Item::Permission {
                    status: Status::Pending,
                    ..
                } | Item::Question {
                    status: Status::Pending,
                    ..
                }
            )
        })
    }

    pub fn running_tasks(&self) -> bool {
        self.tasks
            .values()
            .any(|t| t.status == "running" || t.status == "pending")
    }

    pub fn turn_running(&self) -> bool {
        !matches!(self.turn, None | Some(TurnState::Idle))
    }

    /// Work that should continue with no window open.
    pub fn busy(&self) -> bool {
        self.turn_running()
            || self.running_tasks()
            || self.pending_prompt()
            || !self.queue.is_empty()
            || self.resume_at.is_some()
    }

    /// Everything a newly attached client needs, in order.
    pub fn replay(&self) -> Vec<ConvEvent> {
        let mut out = vec![
            ConvEvent::Reset {
                items: self.items.clone(),
            },
            ConvEvent::Session {
                info: self.session.clone(),
            },
            ConvEvent::Limits {
                limits: self.limits.clone(),
            },
        ];
        out.extend(
            self.tasks
                .values()
                .map(|t| ConvEvent::Task { task: t.clone() }),
        );
        out.push(ConvEvent::Queue {
            items: self.queue.clone(),
        });
        out.push(ConvEvent::Schedule {
            limited_until: self.limited_until.clone(),
            resume_at: self.resume_at.clone(),
        });
        out.push(ConvEvent::Turn {
            state: self.turn.unwrap_or(TurnState::Idle),
            detail: self.turn_detail.clone(),
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glaude_core::bidi::Dir;
    use glaude_core::model::Block;

    fn block(t: &str) -> Block {
        Block {
            html: t.into(),
            dir: Dir::Ltr,
        }
    }

    #[test]
    fn replays_streamed_state() {
        let mut s = Snapshot::default();
        s.apply(&ConvEvent::Turn {
            state: TurnState::Writing,
            detail: None,
        });
        s.apply(&ConvEvent::Item {
            item: Item::Assistant {
                id: "a".into(),
                blocks: vec![],
                parent: None,
                streaming: true,
            },
        });
        s.apply(&ConvEvent::Blocks {
            id: "a".into(),
            from: 0,
            blocks: vec![block("he")],
        });
        s.apply(&ConvEvent::Blocks {
            id: "a".into(),
            from: 0,
            blocks: vec![block("hello"), block("world")],
        });
        s.apply(&ConvEvent::Session {
            info: SessionInfo {
                model: Some("haiku".into()),
                ..Default::default()
            },
        });
        s.apply(&ConvEvent::Session {
            info: SessionInfo {
                session_id: Some("s1".into()),
                ..Default::default()
            },
        });
        assert!(s.busy());
        let replay = s.replay();
        let ConvEvent::Reset { items } = &replay[0] else {
            panic!()
        };
        let Item::Assistant { blocks, .. } = &items[0] else {
            panic!()
        };
        assert_eq!(blocks.len(), 2);
        let ConvEvent::Session { info } = &replay[1] else {
            panic!()
        };
        assert_eq!(info.model.as_deref(), Some("haiku"));
        assert_eq!(info.session_id.as_deref(), Some("s1"));
        s.apply(&ConvEvent::TurnDone {
            duration_ms: None,
            error: None,
        });
        assert!(!s.busy());
    }

    #[test]
    fn pending_permission_counts_as_busy() {
        let mut s = Snapshot::default();
        s.apply(&ConvEvent::Item {
            item: Item::Question {
                id: "q".into(),
                request_id: "r".into(),
                questions: vec![],
                status: Status::Pending,
                answers: Default::default(),
                tool_use_id: String::new(),
            },
        });
        assert!(s.busy());
        assert!(s.waiting_since.is_some());
    }
}
