use crate::*;
use icons::Icon;
#[derive(Default)]
pub struct Chrome {
    pub profile_open: bool,
    pub profile_rect: Option<Rect>,
    pub targets: std::collections::HashMap<&'static str, Rect>,
    pub overlays: Vec<Rect>,
    pub scope: Option<world::TalkScope>,
    pub call_collapsed: bool,
    pub map_rect: Option<Rect>,
    pub world_rect: Option<Rect>,
    pub notice: String,
    pub notice_until: f64,
}
impl Commons {
    pub fn target(&mut self, id: &'static str, rect: Rect) {
        if cfg!(test) || self.qa {
            self.chrome.targets.insert(id, rect);
        }
    }
    pub fn action(
        &mut self,
        ui: &mut Ui,
        id: &'static str,
        icon: Icon,
        label: &str,
        active: bool,
        enabled: bool,
    ) -> Response {
        let response = icons::button(ui, icon, label, active, enabled);
        self.target(id, response.rect);
        response
    }
    pub fn profile_button(&mut self, ui: &mut Ui) {
        let (r, response) = ui.allocate_exact_size(vec2(204.0, 44.0), Sense::click());
        let p = theme::palette(ui.ctx());
        if response.hovered() || self.chrome.profile_open {
            ui.painter().rect_filled(r, 8, p.soft);
        }
        let look = self.settings.avatar.clone();
        self.sprites.draw_avatar(
            ui.painter(),
            r.left_bottom() + vec2(24.0, -4.0),
            49.0,
            &look,
            (2, false),
            ui.input(|i| i.time),
        );
        ui.painter().circle_filled(
            r.left_bottom() + vec2(37.0, -7.0),
            4.0,
            if self.away {
                p.muted
            } else {
                Color32::from_rgb(75, 177, 130)
            },
        );
        let name = if self.settings.name.chars().count() > 18 {
            format!(
                "{}…",
                self.settings.name.chars().take(17).collect::<String>()
            )
        } else {
            self.settings.name.clone()
        };
        ui.painter().text(
            r.left_top() + vec2(54.0, 12.0),
            Align2::LEFT_CENTER,
            name,
            FontId::proportional(12.0),
            p.text,
        );
        ui.painter().text(
            r.left_bottom() + vec2(54.0, -11.0),
            Align2::LEFT_CENTER,
            if self.away { "Away" } else { "Available" },
            FontId::proportional(10.0),
            p.muted,
        );
        icons::paint(
            ui.painter(),
            Icon::Up,
            Rect::from_center_size(r.right_center() - vec2(13.0, 0.0), Vec2::splat(15.0)),
            p.muted,
        );
        self.chrome.profile_rect = Some(r);
        self.target("profile", r);
        if response
            .on_hover_text("Profile, status and workspace options")
            .clicked()
        {
            self.chrome.profile_open = !self.chrome.profile_open;
        }
    }
    pub fn profile_menu(&mut self, ctx: &Context) {
        if !self.chrome.profile_open {
            return;
        }
        let Some(anchor) = self.chrome.profile_rect else {
            return;
        };

        let shown = Area::new(Id::new("profile menu"))
            .order(Order::Foreground)
            .pivot(Align2::LEFT_BOTTOM)
            .fixed_pos(pos2(anchor.left(), anchor.top() - 8.0))
            .movable(false)
            .show(ctx, |ui| {
                Frame::popup(ui.style())
                    .corner_radius(12)
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.set_width(250.0);
                        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
                        ui.label(RichText::new(&self.settings.name).strong());
                        ui.label(
                            RichText::new(if self.connected {
                                &self.snapshot.space
                            } else {
                                "Local preview"
                            })
                            .size(11.0)
                            .color(theme::palette(ctx).muted),
                        );
                        ui.separator();
                        if self
                            .action(ui, "avatar", Icon::Avatar, "Avatar & profile", false, true)
                            .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Profile);
                        }
                        let status = ui.menu_button("     Status", |ui| {
                            let available = ui.selectable_label(!self.away, "Available");
                            self.target("status_available", available.rect);
                            if available.clicked() {
                                self.away = false;
                                self.send(Command::Away(false));
                                ui.close_menu();
                            }
                            let away = ui.selectable_label(self.away, "Away");
                            self.target("status_away", away.rect);
                            if away.clicked() {
                                self.away = true;
                                self.send(Command::Away(true));
                                ui.close_menu();
                            }
                        });
                        ui.painter().circle_filled(
                            status.response.rect.left_center() + vec2(14.0, 0.0),
                            4.0,
                            if self.away {
                                theme::palette(ctx).muted
                            } else {
                                Color32::from_rgb(75, 177, 130)
                            },
                        );
                        self.target("status_menu", status.response.rect);
                        ui.separator();
                        if self.connected
                            && self
                                .action(
                                    ui,
                                    "administration",
                                    Icon::Settings,
                                    "Administration",
                                    self.is_admin,
                                    true,
                                )
                                .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Administration);
                        }
                        self.view_menu(ui);
                        if self
                            .action(ui, "theme", Icon::Theme, "Theme", false, true)
                            .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Appearance);
                        }
                        ui.separator();
                        if !cfg!(target_arch = "wasm32")
                            && self
                                .action(
                                    ui,
                                    "create_server",
                                    Icon::Server,
                                    "Create server",
                                    false,
                                    true,
                                )
                                .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Host);
                        }
                        if self
                            .action(
                                ui,
                                "join_server",
                                Icon::Join,
                                if self.connected {
                                    "Join / switch server"
                                } else {
                                    "Join server"
                                },
                                false,
                                true,
                            )
                            .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Join);
                        }
                        if self.connected
                            && self
                                .action(
                                    ui,
                                    "server_invite",
                                    Icon::Server,
                                    "Server & invite",
                                    false,
                                    true,
                                )
                                .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Invite);
                        }
                        ui.separator();
                        if self
                            .action(ui, "help", Icon::Help, "Help & controls", false, true)
                            .clicked()
                        {
                            self.chrome.profile_open = false;
                            self.open(Dialog::Help);
                        }
                    });
            });
        let rect = shown.response.rect;
        self.chrome.overlays.push(rect);
        self.target("profile_menu", rect);
        if ctx.input(|i| i.key_pressed(Key::Escape)) {
            self.chrome.profile_open = false;
        }
        if ctx.input(|i| i.pointer.any_pressed()) && !ctx.memory(|m| m.any_popup_open()) {
            if let Some(point) = ctx.pointer_interact_pos() {
                if !rect.contains(point) && !anchor.contains(point) {
                    self.chrome.profile_open = false;
                }
            }
        }
    }
    pub fn floating_profile(&mut self, ctx: &Context) {
        Area::new(Id::new("floating profile"))
            .order(Order::Foreground)
            .anchor(Align2::LEFT_BOTTOM, vec2(10.0, -10.0))
            .movable(false)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .corner_radius(9)
                    .inner_margin(4)
                    .show(ui, |ui| self.profile_button(ui));
            });
    }
    pub fn map_controls(&mut self, ctx: &Context, map: Rect) {
        let width = 264.0_f32.min(map.width() - 24.0);
        let pos = pos2(map.right() - width - 12.0, map.bottom() - 48.0);
        let shown = Area::new(Id::new("map zoom"))
            .order(Order::Foreground)
            .fixed_pos(pos)
            .movable(false)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border))
                    .corner_radius(9)
                    .inner_margin(5)
                    .show(ui, |ui| {
                        ui.set_width(width - 10.0);
                        ui.spacing_mut().item_spacing = vec2(5.0, 0.0);
                        ui.horizontal(|ui| {
                            let r = icons::small(ui, Icon::Minus, "Zoom out");
                            self.target("zoom_out", r.rect);
                            if r.clicked() {
                                self.zoom = (self.zoom / 1.2).max(0.5);
                            }
                            ui.scope(|ui| {
                                ui.spacing_mut().slider_width = 85.0;
                                let r = ui
                                    .add(Slider::new(&mut self.zoom, 0.5..=6.0).show_value(false));
                                self.target("zoom_slider", r.rect);
                            });
                            let r = icons::small(ui, Icon::Plus, "Zoom in");
                            self.target("zoom_in", r.rect);
                            if r.clicked() {
                                self.zoom = (self.zoom * 1.2).min(6.0);
                            }
                            ui.label(
                                RichText::new(format!("{}%", (self.zoom * 100.0) as i32))
                                    .size(10.0)
                                    .color(theme::palette(ctx).muted),
                            );
                            let r = icons::small(ui, Icon::Fit, "Fit map");
                            self.target("zoom_fit", r.rect);
                            if r.clicked() {
                                self.zoom = 1.0;
                                self.camera = Vec2::ZERO;
                            }
                        });
                    });
            });
        self.chrome.overlays.push(shown.response.rect);
        self.target("zoom_controls", shown.response.rect);
    }
    pub fn map_toast(&mut self, ctx: &Context, map: Rect) {
        let (message, error) = if !self.error.is_empty() {
            (self.error.clone(), true)
        } else {
            (self.notice.clone(), false)
        };
        let time = ctx.input(|i| i.time);
        if message != self.chrome.notice {
            self.chrome.notice = message.clone();
            self.chrome.notice_until = time + 5.0;
        }
        if message.is_empty() || (!error && time > self.chrome.notice_until) {
            return;
        }
        let shown = Area::new(Id::new("map notice"))
            .order(Order::Foreground)
            .fixed_pos(pos2(map.left() + 12.0, map.bottom() - 52.0))
            .movable(false)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .stroke(Stroke::new(
                        1.0_f32,
                        if error {
                            Color32::from_rgb(190, 75, 75)
                        } else {
                            theme::palette(ctx).border
                        },
                    ))
                    .corner_radius(8)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.set_max_width((map.width() - 320.0).clamp(180.0, 450.0));
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(message).size(11.0));
                            if icons::small(ui, Icon::Close, "Dismiss notice").clicked() {
                                if error {
                                    self.error.clear();
                                } else {
                                    self.notice.clear();
                                }
                            }
                        });
                    });
            });
        self.chrome.overlays.push(shown.response.rect);
    }
    pub fn overlay_contains(&self, ctx: &Context) -> bool {
        ctx.pointer_hover_pos()
            .is_some_and(|p| self.chrome.overlays.iter().any(|r| r.contains(p)))
            || self.chrome.profile_open
            || self.dialog.is_some()
    }
    pub fn sync_call_display(&mut self) {
        let scope = self.current_scope();
        if self.chrome.scope.as_ref() != Some(&scope) {
            if scope.area.is_some() || self.chrome.scope.as_ref().is_some_and(|s| s.area.is_some())
            {
                self.show_call = scope.area.is_some();
            }
            self.chrome.call_collapsed = false;
            self.chrome.scope = Some(scope);
        }
    }
}
pub fn qa_enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .is_some_and(|q| {
                q.trim_start_matches('?')
                    .split('&')
                    .any(|part| part == "qa=1")
            })
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        cfg!(test)
    }
}
