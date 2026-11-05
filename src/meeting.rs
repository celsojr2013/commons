use crate::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pin {
    Camera(u64),
    Screen(u64),
}
#[derive(Default)]
pub struct Meeting {
    pub active: bool,
    pub page: usize,
    pub pin: Option<Pin>,
    pub gallery: bool,
    pub subscription: Option<(Vec<u64>, Vec<u64>)>,
    pub sent_screen: bool,
}
impl Commons {
    pub fn enter_meeting(&mut self) {
        if self.editor.is_some() {
            self.notice = "Save or cancel the map draft before opening Meeting view.".into();
            return;
        }
        self.path.clear();
        self.walking = false;
        self.send_motion();
        self.meeting.active = true;
        self.show_call = true;
        self.chrome.profile_open = false;
    }
    pub fn call_participants(&self) -> Vec<Person> {
        let scope = self.current_scope();
        self.people()
            .into_iter()
            .filter(|p| p.room < self.maps.len() && p.scope(&self.maps) == scope)
            .collect()
    }
    pub fn sync_pin(&mut self, participants: &[Person]) {
        let local = self.id.unwrap_or(0);
        let available = |pin| match pin {
            Pin::Camera(id) => participants.iter().any(|p| p.id == id),
            Pin::Screen(id) => participants.iter().any(|p| {
                p.id == id
                    && if id == local {
                        self.media.screen_on && self.media.screen_ready
                    } else {
                        p.screen
                    }
            }),
        };
        if self.meeting.pin.is_some_and(|p| !available(p)) {
            self.meeting.pin = None;
            self.meeting.gallery = false;
        }
        if self.meeting.pin.is_none() && !self.meeting.gallery {
            self.meeting.pin = participants
                .iter()
                .find(|p| {
                    if p.id == local {
                        self.media.screen_on && self.media.screen_ready
                    } else {
                        p.screen
                    }
                })
                .map(|p| Pin::Screen(p.id));
        }
        let pages = participants.len().max(1).div_ceil(6);
        self.meeting.page = self.meeting.page.min(pages - 1);
    }
    pub fn update_video_subscription(&mut self) {
        let participants = self.call_participants();
        self.sync_pin(&participants);
        let mut cameras = vec![];
        let mut screens = vec![];
        if self.show_call {
            cameras = participants
                .iter()
                .skip(self.meeting.page * 6)
                .take(6)
                .map(|p| p.id)
                .filter(|id| Some(*id) != self.id)
                .collect();
            match self.meeting.pin {
                Some(Pin::Camera(id)) if Some(id) != self.id => cameras.push(id),
                Some(Pin::Screen(id)) if Some(id) != self.id => screens.push(id),
                _ => {}
            }
        }
        cameras.sort_unstable();
        cameras.dedup();
        let subscription = (cameras, screens);
        if self.meeting.subscription.as_ref() != Some(&subscription) {
            self.send(Command::SubscribeVideo {
                cameras: subscription.0.clone(),
                screens: subscription.1.clone(),
            });
            self.media
                .remote
                .retain(|id, _| subscription.0.contains(id));
            self.media
                .screens
                .retain(|id, _| subscription.1.contains(id));
            self.meeting.subscription = Some(subscription);
        }
    }
    fn video_tile(
        &mut self,
        ui: &mut Ui,
        person: &Person,
        screen: bool,
        size: Vec2,
        interactive: bool,
    ) {
        let (rect, response) = ui.allocate_exact_size(
            size,
            if interactive {
                Sense::click()
            } else {
                Sense::hover()
            },
        );
        let p = theme::palette(ui.ctx());
        let selection = if screen {
            Pin::Screen(person.id)
        } else {
            Pin::Camera(person.id)
        };
        let selected = self.meeting.pin == Some(selection);
        ui.painter().rect_filled(rect, 10, p.raised);
        ui.painter()
            .rect_stroke(rect, 10, Stroke::new(1.0_f32, p.border), StrokeKind::Inside);
        let local = Some(person.id) == self.id || (!self.connected && person.id == 0);
        let video = if screen {
            if local {
                self.media.local_screen.as_ref()
            } else {
                self.media.screens.get(&person.id)
            }
        } else if local && self.media.camera_on {
            self.media.local_video.as_ref()
        } else if !local && person.camera {
            self.media.remote.get(&person.id)
        } else {
            None
        };
        let content = Rect::from_min_max(rect.min + vec2(4.0, 4.0), rect.max - vec2(4.0, 30.0));
        if let Some(view) = video {
            let source = view.texture.size_vec2();
            let scale = (content.width() / source.x).min(content.height() / source.y);
            let image = Rect::from_center_size(content.center(), source * scale);
            ui.painter().image(
                view.texture.id(),
                image,
                Rect::from_min_max(Pos2::ZERO, pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        } else if screen {
            ui.painter().text(
                content.center(),
                Align2::CENTER_CENTER,
                "Waiting for shared screen…",
                FontId::proportional(14.0),
                p.muted,
            );
        } else {
            self.sprites.draw_avatar(
                ui.painter(),
                content.center() + vec2(0.0, content.height().min(180.0) * 0.4),
                content.height().min(180.0),
                &person.avatar,
                (2, false),
                ui.input(|i| i.time),
            );
        }
        ui.painter().text(
            rect.left_bottom() + vec2(10.0, -14.0),
            Align2::LEFT_CENTER,
            format!(
                "{}{}{}",
                person.name,
                if local { " · you" } else { "" },
                if screen {
                    " · screen"
                } else if person.mic {
                    " · mic on"
                } else {
                    ""
                }
            ),
            FontId::proportional(12.0),
            p.text,
        );
        if selected {
            ui.painter()
                .rect_stroke(rect, 10, Stroke::new(2.0_f32, p.accent), StrokeKind::Inside);
        }
        if response.clicked() {
            self.meeting.pin = Some(selection);
            self.meeting.gallery = false;
        }
    }
    pub fn meeting_panel(&mut self, ctx: &Context) {
        let participants = self.call_participants();
        self.sync_pin(&participants);
        CentralPanel::default()
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).background)
                    .inner_margin(16),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new(self.conversation_title()).size(20.0).strong());
                    ui.label(format!("{} participants", participants.len()));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let map = ui.button("Map view");
                        self.target("meeting_map", map.rect);
                        if map.clicked() {
                            self.meeting.active = false;
                        }
                        let gallery = ui.selectable_label(
                            self.meeting.gallery || self.meeting.pin.is_none(),
                            "Gallery",
                        );
                        self.target("meeting_gallery", gallery.rect);
                        if gallery.clicked() {
                            self.meeting.pin = None;
                            self.meeting.gallery = true;
                        }
                    });
                });
                ui.add_space(8.0);
                let local_id = self.id;
                let local_screen_ready = self.media.screen_ready;
                ui.horizontal_wrapped(|ui| {
                    for (index, person) in participants
                        .iter()
                        .filter(|p| p.screen || (Some(p.id) == local_id && local_screen_ready))
                        .enumerate()
                    {
                        let pin = Pin::Screen(person.id);
                        let r = ui.selectable_label(
                            self.meeting.pin == Some(pin),
                            format!("Screen · {}", person.name),
                        );
                        if index == 0 {
                            self.target("screen_pin_first", r.rect);
                        }
                        if index == 1 {
                            self.target("screen_pin_second", r.rect);
                        }
                        if r.clicked() {
                            self.meeting.pin = Some(pin);
                            self.meeting.gallery = false;
                        }
                    }
                });
                let pages = participants.len().max(1).div_ceil(6);
                ui.horizontal(|ui| {
                    let previous = ui.add_enabled(self.meeting.page > 0, Button::new("‹"));
                    self.target("meeting_previous", previous.rect);
                    if previous.clicked() {
                        self.meeting.page -= 1;
                    }
                    ui.label(format!("Page {} / {}", self.meeting.page + 1, pages));
                    let next = ui.add_enabled(self.meeting.page + 1 < pages, Button::new("›"));
                    self.target("meeting_next", next.rect);
                    if next.clicked() {
                        self.meeting.page += 1;
                    }
                    ui.label(
                        RichText::new(
                            "Click a camera to pin it. Your selection is private to you.",
                        )
                        .small()
                        .color(theme::palette(ctx).muted),
                    );
                });
                ui.add_space(8.0);
                let available = ui.available_size();
                let visible: Vec<_> = participants
                    .iter()
                    .skip(self.meeting.page * 6)
                    .take(6)
                    .collect();
                if let Some(pin) = self.meeting.pin {
                    let (id, screen) = match pin {
                        Pin::Camera(id) => (id, false),
                        Pin::Screen(id) => (id, true),
                    };
                    if let Some(person) = participants.iter().find(|p| p.id == id) {
                        self.video_tile(
                            ui,
                            person,
                            screen,
                            vec2(available.x, (available.y - 150.0).max(150.0)),
                            false,
                        );
                    }
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let width = ((available.x - 8.0 * visible.len().saturating_sub(1) as f32)
                            / visible.len().max(1) as f32)
                            .max(65.0);
                        for (index, person) in visible.iter().enumerate() {
                            let before = ui.next_widget_position();
                            self.video_tile(ui, person, false, vec2(width, 130.0), true);
                            if index == 0 {
                                self.target(
                                    "camera_pin_first",
                                    Rect::from_min_size(before, vec2(width, 130.0)),
                                );
                            }
                        }
                    });
                } else {
                    let columns = visible.len().clamp(1, 3);
                    let rows = visible.len().max(1).div_ceil(columns);
                    let size = vec2(
                        (available.x - 8.0 * (columns - 1) as f32) / columns as f32,
                        (available.y - 8.0 * (rows - 1) as f32) / rows as f32,
                    );
                    for row in visible.chunks(columns) {
                        ui.horizontal(|ui| {
                            for person in row {
                                self.video_tile(ui, person, false, size, true);
                            }
                        });
                    }
                }
            });
    }
    pub fn screen_picker(&mut self, ctx: &Context) {
        let mut open = true;
        Window::new("Share a screen or window")
            .open(&mut open)
            .collapsible(false)
            .default_width(520.0)
            .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label("Choose one source. Your webcam stays independent.");
                if ui.button("Refresh screens and windows").clicked() {
                    self.media.refresh_screens();
                }
                ScrollArea::vertical().max_height(340.0).show(ui, |ui| {
                    for source in &self.media.screen_sources {
                        ui.selectable_value(
                            &mut self.media.screen_source,
                            source.id.clone(),
                            &source.name,
                        );
                    }
                });
                if ui
                    .add_enabled(
                        !self.media.screen_source.is_empty(),
                        Button::new("Share selected source"),
                    )
                    .clicked()
                {
                    self.media.start_screen();
                    self.dialog = None;
                    self.enter_meeting();
                }
                if !self.media.error.is_empty() {
                    ui.label(&self.media.error);
                }
            });
        if !open {
            self.dialog = None;
        }
    }
}
