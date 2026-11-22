//! Merging of streaming block updates: the host batches a conversation's
//! events and ships them every few milliseconds, so a fast stream costs one
//! message per frame, not one per token.

use glaude_core::model::ConvEvent;

/// Merge `next` into the queue, collapsing consecutive block updates of the
/// same item.
pub(crate) fn push_merged(queue: &mut Vec<ConvEvent>, next: ConvEvent) {
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
