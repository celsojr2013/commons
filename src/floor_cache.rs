use crate::{
    world::{catalog, RoomMap},
    *,
};
pub struct Floor {
    pub width: usize,
    pub height: usize,
    pub ground: Vec<u16>,
    pub texture: TextureHandle,
}
impl Floor {
    pub fn matches(&self, map: &RoomMap) -> bool {
        self.width == map.width && self.height == map.height && self.ground == map.ground
    }
}
pub fn bake(atlas: &image::RgbaImage, map: &RoomMap) -> image::RgbaImage {
    let width = map.width * 16;
    let mut bytes = vec![0; width * map.height * 16 * 4];
    let source = atlas.as_raw();
    for y in 0..map.height {
        for x in 0..map.width {
            let asset = &catalog()[map.ground[y * map.width + x] as usize];
            for row in 0..16 {
                let source_start =
                    ((asset.y as usize + row) * atlas.width() as usize + asset.x as usize) * 4;
                let target = ((y * 16 + row) * width + x * 16) * 4;
                bytes[target..target + 64]
                    .copy_from_slice(&source[source_start..source_start + 64]);
            }
        }
    }
    image::RgbaImage::from_raw(width as u32, (map.height * 16) as u32, bytes).unwrap()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cached_floor_matches_each_source_pixel_and_rebuilds_only_for_floor_changes() {
        let ctx = Context::default();
        let mut sprites = Sprites::new(&ctx);
        let mut map = RoomMap::blank("Floor", 12, 12, 0);
        for (i, tile) in map.ground.iter_mut().enumerate() {
            *tile = (i % 14) as u16;
        }
        let baked = bake(&sprites.environment_pixels, &map);
        for y in 0..12 {
            for x in 0..12 {
                let a = &catalog()[map.ground[y * 12 + x] as usize];
                for py in 0..16 {
                    for px in 0..16 {
                        assert_eq!(
                            baked.get_pixel((x * 16 + px) as u32, (y * 16 + py) as u32),
                            sprites
                                .environment_pixels
                                .get_pixel(a.x + px as u32, a.y + py as u32)
                        );
                    }
                }
            }
        }
        let id = sprites.floor_texture(&ctx, 0, &map);
        assert_eq!(id, sprites.floor_texture(&ctx, 0, &map));
        map.name = "Renamed".into();
        map.stamp(19, 4, 4, false);
        assert_eq!(id, sprites.floor_texture(&ctx, 0, &map));
        map.ground[0] = 13;
        let changed = sprites.floor_texture(&ctx, 0, &map);
        assert_ne!(changed, id);
        map.resize(16, 12);
        assert_ne!(changed, sprites.floor_texture(&ctx, 0, &map));
        assert_eq!(sprites.floors.len(), 1);
    }
}
