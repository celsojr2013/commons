use crate::{world::RoomMap, Vec2};
use std::collections::VecDeque;
pub const SPEED: f32 = 5.75;
pub fn keyboard(map: &RoomMap, position: &mut [f32; 2], direction: Vec2, seconds: f32) {
    let step = direction.normalized() * SPEED * seconds.clamp(0.0, 0.1);
    let count = (step.length() / 0.1).ceil().max(1.0) as usize;
    for _ in 0..count {
        let x = position[0] + step.x / (count as f32 * map.width as f32);
        let y = position[1] + step.y / (count as f32 * map.height as f32);
        if map.can_stand([x, position[1]]) {
            position[0] = x;
        }
        if map.can_stand([position[0], y]) {
            position[1] = y;
        }
    }
}
pub fn path(map: &RoomMap, position: &mut [f32; 2], points: &mut VecDeque<[f32; 2]>, seconds: f32) {
    let mut budget = SPEED * seconds.clamp(0.0, 0.1);
    while budget > 0.00001 {
        let Some(target) = points.front().copied() else {
            break;
        };
        let delta = Vec2::new(
            (target[0] - position[0]) * map.width as f32,
            (target[1] - position[1]) * map.height as f32,
        );
        let distance = delta.length();
        if distance < 0.00001 {
            *position = target;
            points.pop_front();
            continue;
        }
        let amount = budget.min(distance).min(0.1);
        let step = delta / distance * amount;
        let next = [
            position[0] + step.x / map.width as f32,
            position[1] + step.y / map.height as f32,
        ];
        if !map.can_stand(next) {
            points.clear();
            break;
        }
        *position = next;
        budget -= amount;
        if distance - amount < 0.00001 {
            *position = target;
            points.pop_front();
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn keyboard_and_path_walk_fifteen_percent_faster_at_low_or_high_frame_rates() {
        let map = RoomMap::blank("Walking", 64, 64, 0);
        for fps in [12, 15, 30, 60, 120] {
            let mut position = [0.25, 0.5];
            for _ in 0..fps {
                keyboard(&map, &mut position, Vec2::X, 1.0 / fps as f32);
            }
            assert!(((position[0] - 0.25) * 64.0 - SPEED).abs() < 0.002);
            let mut position = [0.25, 0.5];
            let mut points = (17..32).map(|x| [x as f32 / 64.0, 0.5]).collect();
            for _ in 0..fps {
                path(&map, &mut position, &mut points, 1.0 / fps as f32);
            }
            assert!(((position[0] - 0.25) * 64.0 - SPEED).abs() < 0.002);
        }
        assert!((SPEED / 5.0 - 1.15).abs() < 0.00001);
    }
    #[test]
    fn a_long_frame_cannot_jump_through_a_solid_wall() {
        let mut map = RoomMap::blank("Wall", 12, 12, 0);
        map.stamp(14, 5, 5, true);
        let mut position = [4.8 / 12.0, 5.5 / 12.0];
        for _ in 0..5 {
            keyboard(&map, &mut position, Vec2::X, 0.1);
        }
        assert!(position[0] * 12.0 < 5.0);
        let mut points = VecDeque::from([[6.5 / 12.0, 5.5 / 12.0]]);
        path(&map, &mut position, &mut points, 0.1);
        assert!(points.is_empty());
        assert!(position[0] * 12.0 < 5.0);
    }
}
