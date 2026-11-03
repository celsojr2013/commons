use crate::*;
pub fn zoom_at(
    zoom: &mut f32,
    camera: &mut Vec2,
    center: Pos2,
    pointer: Pos2,
    base: f32,
    wheel: f32,
) {
    let old = (base * *zoom).floor().max(4.0);
    *zoom = (*zoom * (wheel * 0.003).exp()).clamp(0.5, 6.0);
    let new = (base * *zoom).floor().max(4.0);
    let offset = pointer - (center + *camera);
    *camera = pointer - center - offset * (new / old);
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wheel_zoom_anchors_the_pointer_and_clamps() {
        let center = pos2(400.0, 300.0);
        let pointer = pos2(620.0, 410.0);
        let mut z = 1.0;
        let mut camera = vec2(17.0, -31.0);
        let before = (pointer - center - camera) / 32.0;
        zoom_at(&mut z, &mut camera, center, pointer, 32.0, 120.0);
        assert!(z > 1.0);
        let after = (pointer - center - camera) / (32.0 * z).floor();
        assert!((before - after).length() < 0.001);
        zoom_at(&mut z, &mut camera, center, pointer, 32.0, -120.0);
        assert!((z - 1.0).abs() < 0.0001);
        zoom_at(&mut z, &mut camera, center, pointer, 32.0, 100000.0);
        assert_eq!(z, 6.0);
    }
}
