use crate::*;

impl Commons {
    pub fn current_scope(&self) -> world::TalkScope {
        self.maps[self.room].scope(self.room, self.position)
    }

    pub fn conversation_title(&self) -> String {
        if let Some(area) = self.maps[self.room].area_at(self.position) {
            format!("Private · {}", area.name)
        } else {
            format!("Room · {}", self.maps[self.room].name)
        }
    }

    pub fn sync_conversation(&mut self) {
        self.sync_call_display();
        let scope = self.connected.then(|| self.current_scope());

        let changed = self.media.scope() != scope.as_ref();

        if changed {
            self.chat.clear();
        }

        if self.media.change_scope(scope) {
            self.notice =
                "Conversation changed. Mic and camera paused; turn them on when you're ready."
                    .into();

            self.send(network::Command::MediaStatus {
                mic: false,
                camera: false,
            });
        }

        if self.current_scope().area.is_some() {
            self.room_chat = true;
        }
    }

    pub fn poll_media(&mut self, ctx: &Context) {
        self.sync_conversation();

        let before = (self.media.mic_on, self.media.camera_on);

        for packet in self.media.poll(ctx, self.connected) {
            self.send(Command::Media(packet.with_scope(self.current_scope())));
        }

        if before != (self.media.mic_on, self.media.camera_on) {
            self.send_media_status();
        }

        let scope = self.current_scope();

        let ids: Vec<_> = self
            .snapshot
            .people
            .iter()
            .filter(|p| p.scope(&self.maps) == scope)
            .map(|p| p.id)
            .collect();

        self.media.retain_participants(&ids);
        let sharing = self.media.screen_on && self.media.screen_ready;
        if self.meeting.sent_screen != sharing {
            self.send(Command::ShareStatus(sharing));
            self.meeting.sent_screen = sharing;
        }
        self.update_video_subscription();
    }

    fn send_media_status(&self) {
        self.send(Command::ShareStatus(
            self.media.screen_on && self.media.screen_ready,
        ));
        self.send(Command::MediaStatus {
            mic: self.media.mic_on,
            camera: self.media.camera_on,
        });
    }

    pub fn call_bar(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("device toolbar")
            .exact_height(62.0)
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .inner_margin(Margin::symmetric(12, 7))
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    self.profile_button(ui);
                    ui.separator();
                    ui.add_space(8.0);
                    ui.spacing_mut().item_spacing = vec2(7.0, 0.0);
                    if self
                        .action(
                            ui,
                            "mic",
                            icons::Icon::Mic,
                            if self.media.mic_on {
                                "Mic on"
                            } else {
                                "Mic off"
                            },
                            self.media.mic_on,
                            self.connected,
                        )
                        .clicked()
                    {
                        self.media.error.clear();
                        self.media.mic_on = !self.media.mic_on;
                        self.media.mic_test = false;
                        self.media.update_input();
                        self.send_media_status();
                    }
                    if self
                        .action(
                            ui,
                            "camera",
                            icons::Icon::Camera,
                            if self.media.camera_on {
                                "Camera on"
                            } else {
                                "Camera off"
                            },
                            self.media.camera_on,
                            self.connected,
                        )
                        .clicked()
                    {
                        self.media.error.clear();
                        self.media.camera_on = !self.media.camera_on;
                        self.show_call = true;
                        self.media.preview = false;
                        self.media.update_camera();
                        self.send_media_status();
                    }
                    let share = self.action(
                        ui,
                        "screen_share",
                        icons::Icon::View,
                        if self.media.screen_on {
                            "Stop screen"
                        } else {
                            "Share screen"
                        },
                        self.media.screen_on,
                        self.connected,
                    );
                    if share.clicked() {
                        self.media.error.clear();
                        if self.media.screen_on {
                            self.media.stop_screen();
                            self.send(Command::ShareStatus(false));
                        } else if cfg!(target_arch = "wasm32") {
                            self.media.start_screen();
                            self.enter_meeting();
                        } else {
                            self.media.refresh_screens();
                            self.open(Dialog::ScreenPicker);
                        }
                    }
                    if self
                        .action(
                            ui,
                            "speakers",
                            icons::Icon::Speaker,
                            if self.media.listening {
                                "Speakers on"
                            } else {
                                "Speakers off"
                            },
                            self.media.listening,
                            self.connected,
                        )
                        .clicked()
                    {
                        self.media.listening = !self.media.listening;
                        self.media.clear_remote();
                        self.media.update_output();
                    }
                    if self
                        .action(ui, "devices", icons::Icon::Settings, "Devices", false, true)
                        .on_hover_text("Camera, microphone and speaker settings")
                        .clicked()
                    {
                        self.open(Dialog::Devices);
                    }
                    if self
                        .action(
                            ui,
                            "call_toggle",
                            icons::Icon::Call,
                            "Call",
                            self.show_call,
                            self.connected || self.current_scope().area.is_some(),
                        )
                        .clicked()
                    {
                        self.show_call = !self.show_call;
                        if !self.show_call {
                            self.meeting.active = false;
                        }
                    }
                    if !self.media.error.is_empty() && ui.small_button("Device issue").clicked() {
                        self.open(Dialog::Devices);
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.connected {
                                self.conversation_title()
                            } else {
                                "Devices stay off until enabled".into()
                            })
                            .size(10.0)
                            .color(theme::palette(ctx).muted),
                        );
                    });
                });
            });
    }

    fn finish_devices(&mut self) {
        self.media.end_tests();
        if !self.connected {
            self.media.stop();
        }
        self.dialog = None;
    }

    pub fn device_dialog(&mut self, ctx: &Context) {
        let mut open = true;

        Window::new("Camera, microphone & speakers").open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER,Vec2::ZERO).default_width(680.0).show(ctx,|ui|{

 let previous_quality = self.media.preferences.quality;
ui.horizontal(|ui| {
    ui.label("Camera quality");
    for (id, quality) in [("quality_low", media::CameraQuality::Low), ("quality_medium", media::CameraQuality::Medium), ("quality_high", media::CameraQuality::High)] {
        let r = ui.selectable_value(&mut self.media.preferences.quality, quality, quality.label()); self.target(id, r.rect);
    }
});
if self.media.preferences.quality != previous_quality { self.media.update_camera(); }
ui.label(RichText::new("Your devices stay off until you choose to use them.").color(theme::palette(ui.ctx()).muted));
ui.label(RichText::new(&self.media.camera_status).size(11.0).color(theme::palette(ui.ctx()).muted));
if ui.button("Enable camera access / retry discovery").clicked(){self.media.request_camera_access();}

ui.add_space(8.0);

 ui.horizontal_top(|ui|{
ui.vertical(|ui|{
ui.set_width(300.0);
let(r,_)=ui.allocate_exact_size(vec2(300.0,225.0),Sense::hover());
ui.painter().rect_filled(r,10,Color32::from_rgb(48,45,62));
if let Some(view)=&self.media.local_video{
ui.painter().image(view.texture.id(),r,Rect::from_min_max(Pos2::ZERO,pos2(1.0,1.0)),Color32::WHITE);
}
else{
ui.painter().text(r.center(),Align2::CENTER_CENTER,if self.media.preview||self.media.camera_on{
"Starting camera…"}
else{
"Camera is off"}
,FontId::proportional(16.0),Color32::WHITE);
}

 let preview=ui.add_enabled(!self.media.camera_on,Button::new(if self.media.preview{
"Stop camera preview"}
else{
"Preview camera locally"}
));self.target("device_preview",preview.rect);if preview.clicked(){
self.media.error.clear();
self.media.preview = !self.media.preview;
self.media.update_camera();
}

 ui.checkbox(&mut self.media.preferences.mirror,"Mirror my preview");
ui.label(RichText::new("Device tests stay local. Use Camera on in the toolbar to share. Closing this window stops local tests.").size(11.0).color(theme::palette(ui.ctx()).muted));

 }
);
ui.add_space(15.0);
ui.vertical(|ui|{
ui.set_width(330.0);

 ui.label(RichText::new("Microphone").strong());
let old=self.media.preferences.microphone.clone();
ComboBox::from_id_salt("mic device").selected_text(if old.is_empty(){
"System default"}
else{
&old}
).width(310.0).show_ui(ui,|ui|{
ui.selectable_value(&mut self.media.preferences.microphone,String::new(),"System default");
for name in &self.media.devices.microphones{
ui.selectable_value(&mut self.media.preferences.microphone,name.clone(),name);
}
}
);
if old!=self.media.preferences.microphone&&(self.media.mic_on||self.media.mic_test){
self.media.update_input();
}

 ui.add(ProgressBar::new((self.media.level()*5.0).min(1.0)).text(if self.media.mic_test||self.media.mic_on{
if self.media.mic_ready{
"Microphone level · local meter"}
else{
"Starting microphone…"}
}
else{
"Microphone is off"}
));
let test=ui.add_enabled(!self.media.mic_on,Button::new(if self.media.mic_test{
"Stop microphone test"}
else{
"Test microphone locally"}
));self.target("device_mic_test",test.rect);if test.clicked(){
self.media.error.clear();
self.media.mic_test = !self.media.mic_test;
self.media.update_input();
}

 ui.label(RichText::new("Speakers / headphones").strong());
let old=self.media.preferences.speaker.clone();
ComboBox::from_id_salt("speaker device").selected_text(if old.is_empty(){
"System default"}
else{
&old}
).width(310.0).show_ui(ui,|ui|{
ui.selectable_value(&mut self.media.preferences.speaker,String::new(),"System default");
for name in &self.media.devices.speakers{
ui.selectable_value(&mut self.media.preferences.speaker,name.clone(),name);
}
}
);
if old!=self.media.preferences.speaker&&self.media.listening{
self.media.update_output();
}
if ui.add(Slider::new(&mut self.media.preferences.volume,0.0..=1.0).text("Volume")).changed(){
self.media.set_volume();
}
if ui.button("Play speaker test").clicked(){
self.media.test_speaker();
}

 ui.label(RichText::new("Camera").strong());
let old=self.media.preferences.camera.clone();
let current=self.media.devices.cameras.iter().find(|(id,_)|*id==old).or_else(||if old=="0"||old.is_empty(){self.media.devices.cameras.first()}else{None}).map(|(_,name)|name.as_str()).unwrap_or("Default camera");
ComboBox::from_id_salt("camera device").selected_text(current).width(310.0).show_ui(ui,|ui|{
ui.selectable_value(&mut self.media.preferences.camera,"0".into(),"Default camera");
for(id,name)in &self.media.devices.cameras{
ui.selectable_value(&mut self.media.preferences.camera,id.clone(),name);
}
}
);
if old!=self.media.preferences.camera&&(self.media.preview||self.media.camera_on){
self.media.update_camera();
}

 }
);
}
);

 ui.add_space(9.0);
ui.label(RichText::new("Calls stay within your room or private area. Crossing a conversation boundary pauses your mic and camera. Use headphones to prevent echo. Camera sharing uses 320 × 240 video at up to 6 fps.").size(11.0).color(theme::palette(ui.ctx()).muted));

 if !self.media.error.is_empty(){
ui.colored_label(Color32::from_rgb(177,61,61),&self.media.error);
}

 ui.horizontal(|ui|{
let save=primary(ui,"Save device settings");self.target("device_save",save.rect);if save.clicked(){
self.settings.media=self.media.preferences.clone();
self.save_settings();
self.finish_devices();
}
if ui.button("Refresh devices").clicked(){
self.media.refresh_devices();
}
if !cfg!(target_arch="wasm32") && ui.button("Privacy settings").clicked(){
let _=std::process::Command::new("open").arg("x-apple.systempreferences:com.apple.preference.security?Privacy_Camera").spawn();
}
if ui.button("Close").clicked(){
self.finish_devices();
}
}
);

 }
);

        if !open {
            self.finish_devices();
        }
    }

    pub fn call_strip(&mut self, ctx: &Context, map: Rect) {
        if !self.show_call {
            return;
        }
        let scope = self.current_scope();
        let participants: Vec<_> = self
            .people()
            .into_iter()
            .filter(|p| p.scope(&self.maps) == scope)
            .collect();
        let width = ((participants.len().clamp(1, 6) as f32 * 142.0 + 24.0).max(350.0))
            .min((map.width() - 28.0).min(900.0));
        let shown = Area::new(Id::new("integrated call strip"))
            .order(Order::Foreground)
            .fixed_pos(pos2(map.center().x - width / 2.0, map.top() + 12.0))
            .movable(false)
            .show(ctx, |ui| {
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border))
                    .corner_radius(12)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(width - 20.0);
                        ui.spacing_mut().item_spacing = vec2(8.0, 6.0);
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(Vec2::splat(17.0), Sense::hover());
                            icons::paint(
                                ui.painter(),
                                if scope.area.is_some() {
                                    icons::Icon::Lock
                                } else {
                                    icons::Icon::Call
                                },
                                r,
                                theme::palette(ctx).accent,
                            );
                            ui.label(RichText::new(self.conversation_title()).size(12.0).strong());
                            let meeting = ui.button("Meeting view");
                            self.target("meeting_open", meeting.rect);
                            if meeting.clicked() {
                                self.enter_meeting();
                            }
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                let r = icons::small(
                                    ui,
                                    if self.chrome.call_collapsed {
                                        icons::Icon::Down
                                    } else {
                                        icons::Icon::Up
                                    },
                                    if self.chrome.call_collapsed {
                                        "Expand call"
                                    } else {
                                        "Collapse call"
                                    },
                                );
                                self.target("call_collapse", r.rect);
                                if r.clicked() {
                                    self.chrome.call_collapsed = !self.chrome.call_collapsed;
                                }
                                ui.label(
                                    RichText::new(format!("{} here", participants.len()))
                                        .size(10.0)
                                        .color(theme::palette(ctx).muted),
                                );
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            for person in participants.iter().filter(|p| p.screen) {
                                if ui.button(format!("Screen · {}", person.name)).clicked() {
                                    self.meeting.pin = Some(meeting::Pin::Screen(person.id));
                                    self.meeting.gallery = false;
                                    self.enter_meeting();
                                }
                            }
                        });
                        if !self.chrome.call_collapsed {
                            ScrollArea::horizontal()
                                .id_salt("call participants")
                                .max_height(128.0)
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        let content = participants.len() as f32 * 128.0 - 8.0;
                                        if content < width - 20.0 {
                                            ui.add_space((width - 20.0 - content).max(0.0) / 2.0);
                                        }
                                        for person in
                                            participants.iter().skip(self.meeting.page * 6).take(6)
                                        {
                                            ui.vertical(|ui| {
                                                ui.set_width(120.0);
                                                let (r, _) = ui.allocate_exact_size(
                                                    vec2(120.0, 90.0),
                                                    Sense::hover(),
                                                );
                                                ui.painter().rect_filled(
                                                    r,
                                                    7,
                                                    theme::palette(ctx).raised,
                                                );
                                                let local = Some(person.id) == self.id
                                                    || (!self.connected && person.id == 0);
                                                let video = if local && self.media.camera_on {
                                                    self.media.local_video.as_ref()
                                                } else if !local && person.camera {
                                                    self.media.remote.get(&person.id)
                                                } else {
                                                    None
                                                };
                                                if let Some(view) = video {
                                                    ui.painter().image(
                                                        view.texture.id(),
                                                        r,
                                                        Rect::from_min_max(
                                                            Pos2::ZERO,
                                                            pos2(1.0, 1.0),
                                                        ),
                                                        Color32::WHITE,
                                                    );
                                                } else {
                                                    self.sprites.draw_avatar(
                                                        ui.painter(),
                                                        r.center_bottom() - vec2(0.0, 9.0),
                                                        90.0,
                                                        &person.avatar,
                                                        (2, false),
                                                        ctx.input(|i| i.time),
                                                    );
                                                }
                                                ui.horizontal(|ui| {
                                                    let (r, _) = ui.allocate_exact_size(
                                                        Vec2::splat(13.0),
                                                        Sense::hover(),
                                                    );
                                                    icons::paint(
                                                        ui.painter(),
                                                        icons::Icon::Mic,
                                                        r,
                                                        if person.mic {
                                                            theme::palette(ctx).accent
                                                        } else {
                                                            theme::palette(ctx).muted
                                                        },
                                                    );
                                                    ui.add(
                                                        Label::new(
                                                            RichText::new(format!(
                                                                "{}{}",
                                                                person.name,
                                                                if local { " · you" } else { "" }
                                                            ))
                                                            .size(10.0),
                                                        )
                                                        .truncate(),
                                                    );
                                                });
                                            });
                                        }
                                    });
                                });
                        }
                    });
            });
        let rect = shown.response.rect;
        self.chrome.overlays.push(rect);
        self.target("call_strip", rect);
    }

    pub fn join_address(&mut self, address: String) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = address;
            self.connect(browser::origin());
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let endpoint = match network::Endpoint::parse(&address) {
                Ok(e) => e,
                Err(e) => {
                    self.error = e;

                    return;
                }
            };

            let own = self.server.as_ref().is_some_and(|s| {
                (s.address.port() == endpoint.port || s.web_port() == Some(endpoint.port))
                    && (endpoint.is_local() || endpoint.host == network::local_address())
            });

            let same = self.connected
                && network::Endpoint::parse(&self.connected_address).is_ok_and(|e| {
                    e.port == endpoint.port
                        && (e.host == endpoint.host || (e.is_local() && endpoint.is_local()))
                });

            if self.connected && (own || same) {
                self.error.clear();

                self.notice =
                "You're already connected to this server. Create server joins you automatically."
                    .into();

                self.dialog = None;

                return;
            }

            if !own {
                self.server = None;
            }

            let target = if ["http://", "https://", "ws://", "wss://"]
                .iter()
                .any(|p| address.starts_with(p))
            {
                address
            } else {
                endpoint.display()
            };
            self.settings.address = target.clone();
            self.connect(target);
        }
    }

    pub fn host_here(&mut self) {
        let endpoint = match network::Endpoint::parse(&self.settings.address) {
            Ok(e) if e.is_local() => e,
            _ => {
                self.error = "Use localhost and a port to host on this computer.".into();

                return;
            }
        };

        if self.settings.name.trim().is_empty() {
            self.error = "Choose a name first.".into();

            return;
        }

        if self.server.is_some() {
            self.join_address(endpoint.display());

            return;
        }

        match self.create_host(&format!("0.0.0.0:{}", endpoint.port)) {
            Ok(server) => {
                self.server = Some(server);

                self.connect(endpoint.display());
            }

            Err(e) => self.error = e,
        }
    }
}
