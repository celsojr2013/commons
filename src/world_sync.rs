use super::Snapshot;
use crate::world::RoomMap;
#[derive(Default)]
pub struct WorldCache {
    revision: Option<u64>,
    rooms: Vec<RoomMap>,
}
impl WorldCache {
    pub fn hydrate(&mut self, state: &mut Snapshot) {
        if state.rooms.len() == 4 && state.rooms.iter().all(RoomMap::validate) {
            if self.revision != Some(state.world_revision) {
                self.revision = Some(state.world_revision);
                self.rooms = state.rooms.clone();
            }
        } else if state.rooms.is_empty() && self.revision == Some(state.world_revision) {
            state.rooms = self.rooms.clone();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_world_updates_are_reused_but_never_substituted_for_an_unknown_revision() {
        let mut cache = WorldCache::default();
        let mut first = Snapshot {
            rooms: crate::world::default_world(),
            world_revision: 9,
            ..Default::default()
        };
        cache.hydrate(&mut first);
        let mut delta = Snapshot {
            world_revision: 9,
            ..Default::default()
        };
        cache.hydrate(&mut delta);
        assert_eq!(delta.rooms, first.rooms);
        let mut unknown = Snapshot {
            world_revision: 10,
            ..Default::default()
        };
        cache.hydrate(&mut unknown);
        assert!(unknown.rooms.is_empty());
    }
}
