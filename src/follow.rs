use crate::*;

/// Move only when the walking character leaves the comfortable inner view.
/// Map-edge limits include room for the sprite above its feet and its name below.
pub fn track(camera: &mut Vec2, viewport: Rect, size: Vec2, position: [f32; 2], cell: f32) {
    let inset = vec2(
        (viewport.width() * 0.22)
            .max(cell * 1.4)
            .min(viewport.width() * 0.4),
        (viewport.height() * 0.22)
            .max(cell * 2.4)
            .min(viewport.height() * 0.4),
    );
    let safe = viewport.shrink2(inset);
    let feet =
        viewport.center() + *camera - size * 0.5 + vec2(position[0] * size.x, position[1] * size.y);
    *camera += feet.clamp(safe.min, safe.max) - feet;
    for axis in 0..2 {
        if size[axis] <= viewport.size()[axis] {
            camera[axis] = 0.0;
        } else {
            let half_overflow = (size[axis] - viewport.size()[axis]) * 0.5;
            let leading = if axis == 1 {
                cell * 2.4 + 12.0
            } else {
                cell * 1.4 + 12.0
            };
            let trailing = if axis == 1 {
                cell * 0.5 + 28.0
            } else {
                leading
            };
            camera[axis] = camera[axis].clamp(-half_overflow - trailing, half_overflow + leading);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tracking_keeps_all_map_edges_visible_and_stops_at_the_room() {
        for viewport_size in [vec2(900.0, 700.0), vec2(420.0, 760.0)] {
            let viewport = Rect::from_min_size(pos2(220.0, 50.0), viewport_size);
            let size = vec2(2400.0, 2400.0);
            let mut camera = Vec2::ZERO;
            for position in [
                [0.5, 0.5],
                [0.99, 0.5],
                [0.99, 0.99],
                [0.01, 0.99],
                [0.01, 0.01],
            ] {
                track(&mut camera, viewport, size, position, 40.0);
                let feet = viewport.center() + camera - size * 0.5
                    + vec2(position[0] * size.x, position[1] * size.y);
                assert!(
                    viewport.shrink(30.0).contains(feet),
                    "{position:?}: {feet:?}"
                );
                assert!(viewport.contains(feet - vec2(0.0, 96.0)));
                assert!(camera.x.abs() <= (size.x - viewport.width()) * 0.5 + 68.01);
                assert!(camera.y.abs() <= (size.y - viewport.height()) * 0.5 + 108.01);
            }
            let settled = camera;
            track(&mut camera, viewport, size, [0.01, 0.01], 40.0);
            assert_eq!(camera, settled);
            track(&mut camera, viewport, viewport_size * 0.8, [0.8, 0.8], 12.0);
            assert_eq!(camera, Vec2::ZERO);
        }
    }
}
