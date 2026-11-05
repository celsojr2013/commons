use crate::{network::Person, world::RoomMap};
use std::collections::HashMap;
struct Track {
    room: usize,
    from: [f32; 2],
    target: [f32; 2],
    at: f64,
}
impl Track {
    fn sample(&self, time: f64) -> [f32; 2] {
        let t = ((time - self.at) / 0.04).clamp(0.0, 1.0) as f32;
        [
            self.from[0] + (self.target[0] - self.from[0]) * t,
            self.from[1] + (self.target[1] - self.from[1]) * t,
        ]
    }
}
#[derive(Default)]
pub struct Motion {
    tracks: HashMap<u64, Track>,
}
impl Motion {
    pub fn update(&mut self, people: &[Person], maps: &[RoomMap], time: f64) {
        self.tracks
            .retain(|id, _| people.iter().any(|p| p.id == *id));
        for p in people {
            let target = [p.x, p.y];
            let from = self
                .tracks
                .get(&p.id)
                .filter(|t| t.room == p.room && time - t.at <= 0.075)
                .map(|t| t.sample(time))
                .filter(|old| {
                    ((old[0] - p.x) * maps[p.room].width as f32)
                        .hypot((old[1] - p.y) * maps[p.room].height as f32)
                        < 3.0
                })
                .unwrap_or(target);
            if self
                .tracks
                .get(&p.id)
                .is_some_and(|t| t.room == p.room && t.target == target)
            {
                continue;
            }
            self.tracks.insert(
                p.id,
                Track {
                    room: p.room,
                    from,
                    target,
                    at: time,
                },
            );
        }
    }
    pub fn sample(&self, id: u64, room: usize, position: [f32; 2], time: f64) -> [f32; 2] {
        self.tracks
            .get(&id)
            .filter(|t| t.room == room)
            .map(|t| t.sample(time))
            .unwrap_or(position)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn motion_interpolates_without_moving_authoritative_positions_or_lerping_between_rooms() {
        let maps = crate::world::default_world();
        let mut motion = Motion::default();
        let mut person = Person {
            id: 1,
            name: "Guest".into(),
            room: 0,
            x: 0.5,
            y: 0.5,
            color: 0,
            away: false,
            avatar: Default::default(),
            direction: 3,
            walking: true,
            mic: false,
            camera: false,
            screen: false,
        };
        motion.update(&[person.clone()], &maps, 1.0);
        person.x = 0.51;
        motion.update(&[person.clone()], &maps, 1.04);
        let halfway = motion.sample(1, 0, [person.x, person.y], 1.06);
        assert!((halfway[0] - 0.505).abs() < 0.0001);
        person.x = 0.52;
        motion.update(&[person.clone()], &maps, 1.2);
        assert_eq!(motion.sample(1, 0, [person.x, person.y], 1.2), [0.52, 0.5]);
        assert_eq!(person.x, 0.52);
        person.room = 1;
        person.x = 0.7;
        motion.update(&[person.clone()], &maps, 1.07);
        assert_eq!(motion.sample(1, 1, [person.x, person.y], 1.07), [0.7, 0.5]);
    }
}
