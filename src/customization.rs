use crate::*;
use avatars::{CLOTH_COLORS, HAIR_COLORS, SKINS};
impl Commons {
    pub fn avatar_dialog(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Make yourself, pixel by pixel")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .default_width(720.0)
            .show(ctx, |ui| {
                ui.add_space(8.0);
                ui.label(
                    RichText::new("A little character. A lot of personality.")
                        .color(theme::palette(ui.ctx()).muted),
                );
                ui.add_space(10.0);
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(240.0);
                        let (r, _) = ui.allocate_exact_size(vec2(240.0, 300.0), Sense::hover());
                        let p = ui.painter_at(r);
                        p.rect_filled(r, 12, Color32::from_rgb(49, 44, 66));
                        for y in 0..5 {
                            for x in 0..6 {
                                let rr = Rect::from_min_size(
                                    r.left_bottom() + vec2(x as f32 * 40.0, -40.0 * (y + 1) as f32),
                                    Vec2::splat(40.0),
                                );
                                self.sprites.tile(&p, 3, rr);
                            }
                        }
                        p.rect_filled(
                            Rect::from_center_size(r.center() + vec2(0.0, 81.0), vec2(65.0, 15.0)),
                            9,
                            Color32::from_black_alpha(60),
                        );
                        self.sprites.draw_avatar(
                            &p,
                            r.center() + vec2(0.0, 86.0),
                            250.0,
                            &self.avatar_draft,
                            (self.preview_direction, self.preview_walking),
                            ctx.input(|i| i.time),
                        );
                        p.text(
                            r.center_top() + vec2(0.0, 27.0),
                            Align2::CENTER_CENTER,
                            "CHARACTER PREVIEW",
                            FontId::proportional(10.0),
                            Color32::from_rgb(194, 182, 213),
                        );
                        ui.horizontal(|ui| {
                            ui.selectable_value(&mut self.preview_walking, false, "Idle");
                            ui.selectable_value(&mut self.preview_walking, true, "Walk");
                            if ui.button("Turn").clicked() {
                                self.preview_direction = (self.preview_direction + 1) % 4;
                            }
                        });
                        ui.add_space(7.0);
                        if ui.button("Surprise me").clicked() {
                            self.avatar_draft = Avatar {
                                body: rand::random::<u8>() as usize % 2,
                                bottom: rand::random::<u8>() as usize % 3,
                                skin: rand::random::<u8>() as usize % 8,
                                face: rand::random::<u8>() as usize % 3,
                                hair: rand::random::<u8>() as usize % 10,
                                hair_color: rand::random::<u8>() as usize % 8,
                                shirt: rand::random::<u8>() as usize % 6,
                                shirt_color: rand::random::<u8>() as usize % 8,
                                pants_color: rand::random::<u8>() as usize % 8,
                                accessory: rand::random::<u8>() as usize % 7,
                                accessory_color: rand::random::<u8>() as usize % 8,
                            };
                        }
                        ui.label(
                            RichText::new(
                                "Four directions · idle + walk\nLayered LPC sprite animation",
                            )
                            .size(11.0)
                            .color(theme::palette(ui.ctx()).muted),
                        );
                        if ui.link("Export my spritesheet").clicked() {
                            #[cfg(target_arch = "wasm32")]
                            match browser::export(self) {
                                Ok(()) => self.notice = "Avatar and art credits downloaded.".into(),
                                Err(e) => self.error = e,
                            }
                            #[cfg(not(target_arch = "wasm32"))]
                            {
                                let stamp = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis();
                                let path =
                                    data_dir().join(format!("exports/my-avatar-{stamp}.png"));
                                let result = std::fs::create_dir_all(path.parent().unwrap())
                                    .map_err(|e| e.to_string())
                                    .and_then(|_| {
                                        self.sprites
                                            .compose(&self.avatar_draft)
                                            .save(&path)
                                            .map_err(|e| e.to_string())
                                    });
                                let result = result.and_then(|_| {
                                    let folder = path.parent().unwrap();
                                    std::fs::write(
                                        folder.join("ART-CREDITS.txt"),
                                        include_bytes!("../assets/ASSET_CREDITS.txt"),
                                    )
                                    .map_err(|e| e.to_string())?;
                                    std::fs::write(
                                        folder.join("CC-BY-SA-4.0.txt"),
                                        include_bytes!("../assets/LICENSES/CC-BY-SA-4.0.txt"),
                                    )
                                    .map_err(|e| e.to_string())?;
                                    Ok(())
                                });
                                match result {
                                    Ok(_) => {
                                        self.notice =
                                            "Spritesheet exported. Use Open exports to find it."
                                                .into()
                                    }
                                    Err(e) => self.error = e,
                                }
                            }
                        }
                        if !cfg!(target_arch = "wasm32") && ui.link("Open exports").clicked() {
                            let folder = data_dir().join("exports");
                            let _ = std::fs::create_dir_all(&folder);
                            let _ = std::process::Command::new("open").arg(folder).spawn();
                        }
                        if ui.link("Art & credits").clicked() {
                            self.dialog = Some(Dialog::Credits);
                        }
                    });
                    ui.add_space(16.0);
                    ui.vertical(|ui| {
                        ui.set_width(420.0);
                        ScrollArea::vertical()
                            .scroll_bar_visibility(scroll_area::ScrollBarVisibility::AlwaysVisible)
                            .max_height((ctx.screen_rect().height() - 330.0).clamp(280.0, 570.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new("DISPLAY NAME")
                                        .size(10.0)
                                        .color(theme::palette(ui.ctx()).muted)
                                        .strong(),
                                );
                                ui.add_enabled(
                                    !self.connected,
                                    TextEdit::singleline(&mut self.profile_name)
                                        .char_limit(32)
                                        .desired_width(f32::INFINITY),
                                );
                                if self.connected {
                                    ui.label(
                                        RichText::new(
                                            "Your outfit updates live. Leave the server to rename.",
                                        )
                                        .size(10.0)
                                        .color(theme::palette(ui.ctx()).muted),
                                    );
                                }
                                section(ui, "Body");
                                ui.push_id("body", |ui| {
                                    ui.horizontal_wrapped(|ui| {
                                        let r = ui.selectable_value(
                                            &mut self.avatar_draft.body,
                                            0,
                                            "Masculine",
                                        );
                                        self.target("body_masculine", r.rect);
                                        let r = ui.selectable_value(
                                            &mut self.avatar_draft.body,
                                            1,
                                            "Feminine",
                                        );
                                        self.target("body_feminine", r.rect);
                                    });
                                });
                                if self.avatar_draft.body == 0 && self.avatar_draft.bottom == 3 {
                                    self.avatar_draft.bottom = 0;
                                }
                                section(ui, "Skin");
                                swatches(ui, &self.sprites, 0, &SKINS, &mut self.avatar_draft.skin);
                                section(ui, "Expression");
                                choices(
                                    ui,
                                    "face",
                                    &["Neutral", "Happy", "Serious"],
                                    &mut self.avatar_draft.face,
                                );
                                section(ui, "Hair");
                                choices(
                                    ui,
                                    "hair",
                                    &[
                                        "Bob",
                                        "Afro",
                                        "Long",
                                        "Tousled",
                                        "Ponytail",
                                        "Bald",
                                        "Bun",
                                        "Braid",
                                        "Long curls",
                                        "Pigtails",
                                    ],
                                    &mut self.avatar_draft.hair,
                                );
                                swatches(
                                    ui,
                                    &self.sprites,
                                    1,
                                    &HAIR_COLORS,
                                    &mut self.avatar_draft.hair_color,
                                );
                                section(ui, "Clothing");
                                choices(
                                    ui,
                                    "shirt",
                                    if self.avatar_draft.body == 1 {
                                        &[
                                            "T-shirt",
                                            "Long sleeves",
                                            "Scoop neck",
                                            "Blouse",
                                            "Tunic",
                                            "Dress",
                                        ]
                                    } else {
                                        &[
                                            "T-shirt",
                                            "Long sleeves",
                                            "Scoop neck",
                                            "Vest",
                                            "Formal shirt",
                                            "Cardigan",
                                        ]
                                    },
                                    &mut self.avatar_draft.shirt,
                                );
                                swatches(
                                    ui,
                                    &self.sprites,
                                    2,
                                    &CLOTH_COLORS,
                                    &mut self.avatar_draft.shirt_color,
                                );
                                section(ui, "Bottoms");
                                choices(
                                    ui,
                                    "bottom",
                                    if self.avatar_draft.body == 1 {
                                        &["Trousers", "Shorts", "Skirt", "Long skirt"]
                                    } else {
                                        &["Trousers", "Shorts", "Skirt"]
                                    },
                                    &mut self.avatar_draft.bottom,
                                );
                                swatches(
                                    ui,
                                    &self.sprites,
                                    2,
                                    &[
                                        "navy", "gray", "brown", "green", "black", "blue", "white",
                                        "maroon",
                                    ],
                                    &mut self.avatar_draft.pants_color,
                                );
                                section(ui, "Accessories");
                                choices(
                                    ui,
                                    "accessory",
                                    &[
                                        "None",
                                        "Glasses",
                                        "Wizard hat",
                                        "Top hat",
                                        "Headscarf",
                                        "Tiara",
                                        "Bandana",
                                    ],
                                    &mut self.avatar_draft.accessory,
                                );
                                if self.avatar_draft.accessory > 1 {
                                    swatches(
                                        ui,
                                        &self.sprites,
                                        2,
                                        &CLOTH_COLORS,
                                        &mut self.avatar_draft.accessory_color,
                                    );
                                }
                            });
                    });
                });
                ui.separator();
                ui.horizontal(|ui| {
                    let save = primary(ui, "Save my character");
                    self.target("avatar_save", save.rect);
                    if save.clicked() {
                        self.settings.avatar = self.avatar_draft.clone();
                        if !self.connected {
                            self.settings.name = if self.profile_name.trim().is_empty() {
                                "You".into()
                            } else {
                                self.profile_name.trim().to_owned()
                            };
                        }
                        self.save_settings();
                        self.send(Command::Style(self.settings.avatar.clone()));
                        self.notice = "Your character is saved. Looking good!".into();
                        self.dialog = None;
                    }
                    if ui.button("Cancel").clicked() {
                        self.dialog = None;
                    }
                    ui.label(
                        RichText::new("Changes appear for everyone when you save.")
                            .size(10.0)
                            .color(theme::palette(ui.ctx()).muted),
                    );
                });
            });
        if !open {
            self.dialog = None;
        }
    }
    pub fn credits_dialog(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Pixel art & credits").open(&mut open).resizable(true).default_width(680.0).anchor(Align2::CENTER_CENTER,Vec2::ZERO).show(ctx,|ui|{
ui.label(RichText::new("A world made possible by generous artists.").strong().size(18.0));
ui.hyperlink_to("Kenney · environment tiles (CC0)","https://kenney.nl/assets/roguelike-indoors");
ui.hyperlink_to("Liberated Pixel Cup · layered character art","https://github.com/LiberatedPixelCup/Universal-LPC-Spritesheet-Character-Generator");
ScrollArea::vertical().max_height(480.0).show(ui,|ui|{
ui.label(include_str!("../assets/ASSET_CREDITS.txt"));
}
);
if ui.button("Back to my character").clicked(){
self.dialog = Some(Dialog::Profile);
}
}
);
        if !open {
            self.dialog = None;
        }
    }
}
fn section(ui: &mut Ui, name: &str) {
    ui.add_space(5.0);
    ui.label(RichText::new(name).strong().size(12.0));
}
fn choices(ui: &mut Ui, id: &str, labels: &[&str], selection: &mut usize) {
    ui.push_id(id, |ui| {
        ui.horizontal_wrapped(|ui| {
            for (i, n) in labels.iter().enumerate() {
                ui.selectable_value(selection, i, *n);
            }
        });
    });
}
fn swatches(ui: &mut Ui, sprites: &Sprites, kind: usize, names: &[&str], selected: &mut usize) {
    ui.horizontal(|ui| {
        for (i, name) in names.iter().enumerate() {
            let color = sprites.swatch(kind, name);
            let (r, response) = ui.allocate_exact_size(vec2(29.0, 29.0), Sense::click());
            ui.painter().circle_filled(r.center(), 11.0, color);
            if i == *selected {
                ui.painter().circle_stroke(
                    r.center(),
                    14.0,
                    Stroke::new(2.0, theme::palette(ui.ctx()).accent),
                );
            }
            if response.on_hover_text(name.replace('_', " ")).clicked() {
                *selected = i;
            }
        }
    });
}
