use super::Event;
use std::collections::VecDeque;
#[derive(Default)]
pub struct EventQueue {
    events: VecDeque<Event>,
}
impl EventQueue {
    pub fn push(&mut self, mut event: Event) {
        // Retain a pending world update when several movement packets arrive before the next frame.
        if let Event::State(ref mut incoming) = event {
            if incoming.rooms.is_empty() {
                if let Some(Event::State(previous)) = self
                    .events
                    .iter_mut()
                    .find(|e| matches!(e, Event::State(_)))
                {
                    if previous.world_revision == incoming.world_revision {
                        incoming.rooms = std::mem::take(&mut previous.rooms);
                    }
                }
            }
        }
        match &event {
            Event::State(_) => self.events.retain(|e| !matches!(e, Event::State(_))),
            Event::ConversationChanged(_) => self
                .events
                .retain(|e| !matches!(e, Event::Media { .. } | Event::ConversationChanged(_))),
            Event::Media { sender, packet, .. } if packet.kind != super::MediaKind::Audio => {
                self.events.retain(|e| !matches!(e, Event::Media { sender: previous, packet: old, .. } if previous == sender && old.kind == packet.kind));
                if self.events.len() >= 64 {
                    return;
                }
            }
            Event::Media { .. } if self.events.len() >= 64 => return,
            _ => {}
        }
        if self.events.len() >= 128 {
            let index = self
                .events
                .iter()
                .position(|e| matches!(e, Event::Media { .. } | Event::Notice(_)))
                .unwrap_or(0);
            self.events.remove(index);
        }
        self.events.push_back(event);
    }
    pub fn drain(&mut self) -> std::collections::vec_deque::IntoIter<Event> {
        std::mem::take(&mut self.events).into_iter()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        network::{MediaKind, MediaPacket, Snapshot},
        world::TalkScope,
    };
    #[test]
    fn coalescing_motion_preserves_the_latest_pending_room_revision() {
        let mut q = EventQueue::default();
        let mut rooms = crate::world::default_world();
        rooms[0].name = "Latest office".into();
        q.push(Event::State(Snapshot {
            rooms: rooms.clone(),
            world_revision: 7,
            ..Default::default()
        }));
        for _ in 0..30 {
            q.push(Event::State(Snapshot {
                world_revision: 7,
                ..Default::default()
            }));
        }
        let events: Vec<_> = q.drain().collect();
        assert_eq!(events.len(), 1);
        match &events[0] {
            Event::State(state) => assert_eq!(state.rooms, rooms),
            _ => panic!("expected state"),
        }
    }
    #[test]
    fn background_tab_queue_is_bounded_and_preserves_boundaries_and_errors() {
        let mut q = EventQueue::default();
        let scope = TalkScope {
            room: 0,
            area: None,
        };
        for _ in 0..400 {
            q.push(Event::Media {
                sender: 1,
                scope: scope.clone(),
                packet: MediaPacket::from_bytes(MediaKind::Audio, &[0; 640]),
            });
        }
        assert!(q.events.len() <= 64);
        q.push(Event::ConversationChanged(scope));
        assert!(q.events.iter().all(|e| !matches!(e, Event::Media { .. })));
        for _ in 0..400 {
            q.push(Event::State(Snapshot::default()));
        }
        assert_eq!(
            q.events
                .iter()
                .filter(|e| matches!(e, Event::State(_)))
                .count(),
            1
        );
        q.push(Event::Error("Password incorrect".into()));
        q.push(Event::Disconnected);
        let drained: Vec<_> = q.drain().collect();
        assert!(drained
            .iter()
            .any(|e| matches!(e, Event::ConversationChanged(_))));
        assert!(drained.iter().any(|e| matches!(e, Event::Error(_))));
        assert!(q.events.is_empty());
    }
}
