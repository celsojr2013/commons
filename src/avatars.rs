use eframe::egui::*;
use image::{Rgba, RgbaImage};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(default)]
pub struct Avatar {
    pub body: usize,
    pub bottom: usize,
    pub skin: usize,
    pub face: usize,
    pub hair: usize,
    pub hair_color: usize,
    pub shirt: usize,
    pub shirt_color: usize,
    pub pants_color: usize,
    pub accessory: usize,
    pub accessory_color: usize,
}
impl Default for Avatar {
    fn default() -> Self {
        Self {
            body: 0,
            bottom: 0,
            skin: 0,
            face: 1,
            hair: 3,
            hair_color: 0,
            shirt: 0,
            shirt_color: 0,
            pants_color: 0,
            accessory: 0,
            accessory_color: 0,
        }
    }
}
impl Avatar {
    pub fn valid(&self) -> bool {
        self.body < 2
            && self.bottom < if self.body == 1 { 4 } else { 3 }
            && self.skin < 8
            && self.face < 3
            && self.hair < 10
            && self.hair_color < 8
            && self.shirt < 6
            && self.shirt_color < 8
            && self.pants_color < 8
            && self.accessory < 7
            && self.accessory_color < 8
    }
}
pub const SKINS: [&str; 8] = [
    "light", "amber", "olive", "taupe", "bronze", "brown", "black", "lavender",
];
pub const HAIR_COLORS: [&str; 8] = [
    "black",
    "dark_brown",
    "blonde",
    "ginger",
    "pink",
    "purple",
    "blue",
    "white",
];
pub const CLOTH_COLORS: [&str; 8] = [
    "lavender", "blue", "teal", "green", "orange", "red", "white", "black",
];
fn rgb(s: &str) -> [u8; 3] {
    let s = s.trim_start_matches('#');
    [
        u8::from_str_radix(&s[0..2], 16).unwrap(),
        u8::from_str_radix(&s[2..4], 16).unwrap(),
        u8::from_str_radix(&s[4..6], 16).unwrap(),
    ]
}
pub struct Sprites {
    pub environment: TextureHandle,
    pub(crate) environment_pixels: RgbaImage,
    pub(crate) floors: HashMap<usize, crate::floor_cache::Floor>,
    layers: HashMap<&'static str, RgbaImage>,
    palettes: [HashMap<String, Vec<String>>; 3],
    cache: HashMap<Avatar, (TextureHandle, u64)>,
    clock: u64,
}
macro_rules! layer {
    ($m:expr,$name:literal) => {
        $m.insert(
            $name,
            image::load_from_memory(include_bytes!(concat!(
                "../assets/characters/",
                $name,
                ".png"
            )))
            .unwrap()
            .to_rgba8(),
        );
    };
}
impl Sprites {
    pub fn new(ctx: &Context) -> Self {
        let im = image::load_from_memory(include_bytes!("../assets/environment.png"))
            .unwrap()
            .to_rgba8();
        let environment = ctx.load_texture(
            "environment",
            ColorImage::from_rgba_unmultiplied(
                [im.width() as usize, im.height() as usize],
                im.as_raw(),
            ),
            TextureOptions::NEAREST,
        );
        let mut layers = HashMap::new();
        layer!(layers, "body");
        layer!(layers, "head");
        layer!(layers, "face_neutral");
        layer!(layers, "face_happy");
        layer!(layers, "face_serious");
        layer!(layers, "hair_bob");
        layer!(layers, "hair_afro");
        layer!(layers, "hair_long");
        layer!(layers, "hair_messy");
        layer!(layers, "hair_ponytail");
        layer!(layers, "hair_ponytail_back");
        layer!(layers, "pants");
        layer!(layers, "shirt_tee");
        layer!(layers, "shirt_long");
        layer!(layers, "shirt_scoop");
        layer!(layers, "shoes");
        layer!(layers, "acc_glasses");
        layer!(layers, "acc_wizard");
        layer!(layers, "acc_tophat");
        layer!(layers, "acc_hijab");
        layer!(layers, "f_body");
        layer!(layers, "f_head");
        layer!(layers, "f_face_neutral");
        layer!(layers, "f_face_happy");
        layer!(layers, "f_face_serious");
        layer!(layers, "f_pants");
        layer!(layers, "f_shoes");
        layer!(layers, "f_shirt_tee");
        layer!(layers, "f_shirt_long");
        layer!(layers, "f_shirt_scoop");
        layer!(layers, "f_shirt_blouse");
        layer!(layers, "f_shirt_tunic");
        layer!(layers, "f_shirt_dress");
        layer!(layers, "shirt_vest");
        layer!(layers, "shirt_formal");
        layer!(layers, "shirt_cardigan");
        layer!(layers, "shorts");
        layer!(layers, "f_shorts");
        layer!(layers, "skirt");
        layer!(layers, "f_skirt");
        layer!(layers, "f_long_skirt");
        layer!(layers, "hair_bun");
        layer!(layers, "hair_braid");
        layer!(layers, "hair_braid_back");
        layer!(layers, "hair_curls");
        layer!(layers, "hair_pigtails");
        layer!(layers, "acc_tiara");
        layer!(layers, "acc_bandana");
        layer!(layers, "f_acc_hijab");
        let palettes = [
            serde_json::from_str(include_str!("../assets/palettes/body_ulpc.json")).unwrap(),
            serde_json::from_str(include_str!("../assets/palettes/hair_ulpc.json")).unwrap(),
            serde_json::from_str(include_str!("../assets/palettes/cloth_ulpc.json")).unwrap(),
        ];
        Self {
            environment,
            environment_pixels: im,
            floors: HashMap::new(),
            layers,
            palettes,
            cache: HashMap::new(),
            clock: 0,
        }
    }
    fn recolor(&self, name: &str, kind: usize, from: &str, to: &str) -> RgbaImage {
        let mut layer = self.layers[name].clone();
        let source: Vec<_> = self.palettes[kind][from].iter().map(|s| rgb(s)).collect();
        let target: Vec<_> = self.palettes[kind][to].iter().map(|s| rgb(s)).collect();
        for p in layer.pixels_mut() {
            if p[3] > 0 {
                if let Some(i) = source.iter().position(|c| *c == [p[0], p[1], p[2]]) {
                    *p = Rgba([target[i][0], target[i][1], target[i][2], p[3]]);
                }
            }
        }
        layer
    }
    pub fn compose(&self, look: &Avatar) -> RgbaImage {
        let a = if look.valid() {
            look.clone()
        } else {
            Avatar::default()
        };
        let mut out = RgbaImage::new(576, 512);
        let mut add = |im: &RgbaImage| image::imageops::overlay(&mut out, im, 0, 0);
        if a.hair == 4 {
            add(&self.recolor("hair_ponytail_back", 1, "orange", HAIR_COLORS[a.hair_color]));
        }
        if a.hair == 7 {
            add(&self.recolor("hair_braid_back", 1, "orange", HAIR_COLORS[a.hair_color]));
        }
        add(&self.recolor(
            if a.body == 1 { "f_body" } else { "body" },
            0,
            "light",
            SKINS[a.skin],
        ));
        add(&self.recolor(
            if a.body == 1 { "f_head" } else { "head" },
            0,
            "light",
            SKINS[a.skin],
        ));
        add(&self.recolor(
            if a.body == 1 { "f_shoes" } else { "shoes" },
            2,
            "white",
            "black",
        ));
        add(&self.recolor(
            if a.body == 1 {
                ["f_pants", "f_shorts", "f_skirt", "f_long_skirt"][a.bottom]
            } else {
                ["pants", "shorts", "skirt"][a.bottom]
            },
            2,
            "white",
            [
                "navy", "gray", "brown", "green", "black", "blue", "white", "maroon",
            ][a.pants_color],
        ));
        add(&self.recolor(
            if a.body == 1 {
                [
                    "f_shirt_tee",
                    "f_shirt_long",
                    "f_shirt_scoop",
                    "f_shirt_blouse",
                    "f_shirt_tunic",
                    "f_shirt_dress",
                ][a.shirt]
            } else {
                [
                    "shirt_tee",
                    "shirt_long",
                    "shirt_scoop",
                    "shirt_vest",
                    "shirt_formal",
                    "shirt_cardigan",
                ][a.shirt]
            },
            2,
            "white",
            CLOTH_COLORS[a.shirt_color],
        ));
        add(&self.recolor(
            if a.body == 1 {
                ["f_face_neutral", "f_face_happy", "f_face_serious"][a.face]
            } else {
                ["face_neutral", "face_happy", "face_serious"][a.face]
            },
            0,
            "light",
            SKINS[a.skin],
        ));
        if a.hair != 5 {
            add(&self.recolor(
                [
                    "hair_bob",
                    "hair_afro",
                    "hair_long",
                    "hair_messy",
                    "hair_ponytail",
                    "",
                    "hair_bun",
                    "hair_braid",
                    "hair_curls",
                    "hair_pigtails",
                ][a.hair],
                1,
                "orange",
                HAIR_COLORS[a.hair_color],
            ));
        }
        if a.accessory > 0 {
            let name = if a.accessory == 4 && a.body == 1 {
                "f_acc_hijab"
            } else {
                [
                    "",
                    "acc_glasses",
                    "acc_wizard",
                    "acc_tophat",
                    "acc_hijab",
                    "acc_tiara",
                    "acc_bandana",
                ][a.accessory]
            };
            let mut im = self.recolor(name, 2, "white", CLOTH_COLORS[a.accessory_color]);
            if a.accessory == 2 {
                let target: Vec<_> = self.palettes[2][CLOTH_COLORS[a.accessory_color]]
                    .iter()
                    .map(|s| rgb(s))
                    .collect();
                for p in im.pixels_mut() {
                    if p[3] > 0 {
                        let shade = ((p[0] as f32 * 0.25 + p[1] as f32 * 0.6 + p[2] as f32 * 0.15)
                            / 200.0
                            * 5.0)
                            .clamp(0.0, 5.0) as usize;
                        let c = target[shade];
                        *p = Rgba([c[0], c[1], c[2], p[3]]);
                    }
                }
            }
            add(&im);
        }
        out
    }
    pub fn texture(&mut self, ctx: &Context, avatar: &Avatar) -> TextureId {
        self.clock += 1;
        if let Some((texture, used)) = self.cache.get_mut(avatar) {
            *used = self.clock;
            return texture.id();
        }
        if self.cache.len() >= 96 {
            if let Some(oldest) = self
                .cache
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(look, _)| look.clone())
            {
                self.cache.remove(&oldest);
            }
        }
        let image = self.compose(avatar);
        let texture = ctx.load_texture(
            format!("avatar-{avatar:?}"),
            ColorImage::from_rgba_unmultiplied([576, 512], image.as_raw()),
            TextureOptions::NEAREST,
        );
        let id = texture.id();
        self.cache.insert(avatar.clone(), (texture, self.clock));
        id
    }
    pub fn draw_avatar(
        &mut self,
        p: &Painter,
        feet: Pos2,
        size: f32,
        avatar: &Avatar,
        pose: (u8, bool),
        time: f64,
    ) {
        let (direction, walking) = pose;
        let texture = self.texture(p.ctx(), avatar);
        let column = if walking {
            1 + (time * 8.0) as usize % 8
        } else {
            (time * 1.5) as usize % 2
        };
        let row = usize::from(direction.min(3)) + if walking { 0 } else { 4 };
        let uv = Rect::from_min_max(
            pos2(column as f32 * 64.0 / 576.0, row as f32 / 8.0),
            pos2((column + 1) as f32 * 64.0 / 576.0, (row + 1) as f32 / 8.0),
        );
        let rect = Rect::from_min_size(feet - vec2(size / 2.0, size * 0.88), vec2(size, size));
        p.image(texture, rect, uv, Color32::WHITE);
    }
    pub fn floor_texture(
        &mut self,
        ctx: &Context,
        room: usize,
        map: &crate::world::RoomMap,
    ) -> TextureId {
        if !self.floors.get(&room).is_some_and(|f| f.matches(map)) {
            let image = crate::floor_cache::bake(&self.environment_pixels, map);
            let texture = ctx.load_texture(
                format!("floor-{room}"),
                ColorImage::from_rgba_unmultiplied(
                    [image.width() as usize, image.height() as usize],
                    image.as_raw(),
                ),
                TextureOptions::NEAREST,
            );
            self.floors.insert(
                room,
                crate::floor_cache::Floor {
                    width: map.width,
                    height: map.height,
                    ground: map.ground.clone(),
                    texture,
                },
            );
        }
        self.floors[&room].texture.id()
    }
    pub fn tile(&self, p: &Painter, id: u16, rect: Rect) {
        let Some(a) = crate::world::catalog().get(id as usize) else {
            return;
        };
        let size = self.environment.size_vec2();
        let uv = Rect::from_min_max(
            pos2(a.x as f32 / size.x, a.y as f32 / size.y),
            pos2(
                (a.x + a.width) as f32 / size.x,
                (a.y + a.height) as f32 / size.y,
            ),
        );
        p.image(self.environment.id(), rect, uv, Color32::WHITE);
    }
    pub fn swatch(&self, kind: usize, name: &str) -> Color32 {
        let c = rgb(&self.palettes[kind][name][4]);
        Color32::from_rgb(c[0], c[1], c[2])
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_profiles_remain_valid_and_invalid_indices_are_rejected() {
        let old = r#"{"skin":4,"face":2,"hair":4,"hair_color":3,"shirt":2,"shirt_color":1,"pants_color":5,"accessory":4,"accessory_color":6}"#;
        let look: Avatar = serde_json::from_str(old).unwrap();
        assert!(look.valid());
        assert_eq!(look.body, 0);
        assert_eq!(look.bottom, 0);
        assert_eq!(look.skin, 4);
        assert!(!Avatar {
            body: 2,
            ..look.clone()
        }
        .valid());
        assert!(!Avatar {
            hair: 10,
            ..look.clone()
        }
        .valid());
        assert!(!Avatar {
            body: 0,
            bottom: 3,
            ..look.clone()
        }
        .valid());
        assert!(Avatar {
            body: 1,
            bottom: 3,
            ..look
        }
        .valid());
    }
    #[test]
    fn all_customization_layers_have_visible_animation_frames() {
        let ctx = Context::default();
        let sprites = Sprites::new(&ctx);
        let baseline = sprites.compose(&Avatar::default());
        for i in 0..20 {
            let look = Avatar {
                body: i % 2,
                bottom: if i % 2 == 1 { i % 4 } else { i % 3 },
                skin: i % 8,
                hair: i % 10,
                face: i % 3,
                hair_color: i % 8,
                shirt: i % 6,
                shirt_color: i % 8,
                pants_color: i % 8,
                accessory: i % 7,
                accessory_color: i % 8,
            };
            let sheet = sprites.compose(&look);
            assert_eq!(sheet.dimensions(), (576, 512));
            for row in 0..8 {
                let frame = image::imageops::crop_imm(&sheet, 0, row * 64, 64, 64).to_image();
                assert!(frame.pixels().filter(|p| p[3] > 0).count() > 200);
            }
            if i > 0 {
                assert_ne!(sheet, baseline);
            }
        }
    }
}
