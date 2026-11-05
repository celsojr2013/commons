use crate::*;
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub rooms: bool,
    pub conversation: bool,
    pub calls: bool,
    pub map_only: bool,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            rooms: true,
            conversation: true,
            calls: true,
            map_only: false,
        }
    }
}
impl Commons {
    pub fn toggle_map_only(&mut self) {
        self.chrome.profile_open = false;
        self.settings.view.map_only = !self.settings.view.map_only;
        if self.settings.view.map_only {
            self.people_tab = false;
            self.suspended_dialog = self.dialog.take();
            if self.suspended_dialog == Some(Dialog::Devices) {
                self.media.end_tests();
            }
            if let Some(editor) = self.editor.take() {
                self.suspended_editor = Some((self.room, editor));
            }
        } else {
            self.dialog = self.suspended_dialog.take();
            if let Some((room, editor)) = self.suspended_editor.take() {
                self.room = room;
                self.position = self.maps[room].spawn();
                self.path.clear();
                self.walking = false;
                self.editor = Some(editor);
                self.send_motion();
            }
        }
        self.save_settings();
    }
    pub fn view_menu(&mut self, ui: &mut Ui) {
        let menu = ui.menu_button("     View", |ui| {
            let mut changed = false;
            changed |= ui
                .checkbox(&mut self.settings.view.rooms, "Rooms sidebar")
                .changed();
            changed |= ui
                .checkbox(
                    &mut self.settings.view.conversation,
                    "Chat / editor sidebar",
                )
                .changed();
            changed |= ui
                .checkbox(&mut self.settings.view.calls, "Device toolbar")
                .changed();
            if changed {
                self.save_settings();
            }
            ui.separator();
            if ui.button("Map only · F10").clicked() {
                self.toggle_map_only();
                ui.close_menu();
            }
            if ui.button("Show all panels").clicked() {
                self.settings.view = Preferences::default();
                self.save_settings();
                ui.close_menu();
            }
        });
        icons::paint(
            ui.painter(),
            icons::Icon::View,
            Rect::from_center_size(
                menu.response.rect.left_center() + vec2(14.0, 0.0),
                Vec2::splat(17.0),
            ),
            theme::palette(ui.ctx()).text,
        );
        self.target("view_menu", menu.response.rect);
    }
    pub fn shortcuts(&mut self, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(Key::F10)) {
            self.toggle_map_only();
        }
        if self.settings.view.map_only && ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.toggle_map_only();
        }
    }
    pub fn restore_controls(&mut self, ctx: &Context) {
        Area::new(Id::new("restore controls"))
            .order(Order::Foreground)
            .anchor(Align2::LEFT_TOP, vec2(12.0, 12.0))
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .corner_radius(8)
                    .inner_margin(5)
                    .show(ui, |ui| {
                        if ui.small_button("Show controls · Esc").clicked() {
                            self.toggle_map_only();
                        }
                        if self.media.mic_on || self.media.camera_on || self.media.screen_on {
                            ui.label(if self.media.mic_on && self.media.camera_on {
                                "Mic + camera on"
                            } else if self.media.mic_on {
                                "Mic on"
                            } else {
                                if self.media.screen_on {
                                    "Screen on"
                                } else {
                                    "Camera on"
                                }
                            });
                            if ui.small_button("Stop sharing").clicked() {
                                self.media.pause_sharing();
                                self.send(Command::MediaStatus {
                                    mic: false,
                                    camera: false,
                                });
                            }
                        }
                    });
            });
    }
    pub fn draw_workspace(&mut self, ctx: &Context) {
        self.chrome.map_rect = None;
        self.chrome.world_rect = None;
        self.chrome.targets.clear();
        self.chrome.overlays.clear();
        self.shortcuts(ctx);
        theme::apply(ctx, self.settings.appearance);
        if !self.settings.view.map_only {
            if self.settings.view.calls {
                self.call_bar(ctx);
            } else {
                self.floating_profile(ctx);
            }
            self.header(ctx);
            if self.settings.view.rooms && !self.meeting.active {
                self.sidebar(ctx);
            }
            if self.settings.view.conversation {
                if self.editor.is_some() {
                    self.editor_panel(ctx);
                } else {
                    self.chat_panel(ctx);
                }
            }
        }
        if self.meeting.active {
            self.meeting_panel(ctx);
        } else {
            self.main_panel(ctx);
        }
        if self.settings.view.map_only {
            self.restore_controls(ctx);
        }
        self.profile_menu(ctx);
        self.dialogs(ctx);
    }
}
