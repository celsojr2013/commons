mod administration;
mod avatars;
#[cfg(target_arch = "wasm32")]
mod browser;
mod calls;
#[cfg(not(target_arch = "wasm32"))]
mod camera_backend;
mod chrome;
mod customization;
mod editor;
mod floor_cache;
mod follow;
mod hosting;
mod icons;
#[cfg_attr(target_arch = "wasm32", path = "media_web.rs")]
mod media;
mod media_types;
mod meeting;
mod movement;
mod network;
mod remote_motion;
mod scene;
#[cfg(not(target_arch = "wasm32"))]
mod screen_backend;
mod space_editor;
mod storage;
mod ui_theme;
mod view;
#[cfg(not(target_arch = "wasm32"))]
mod web_host;
mod world;
mod zoom;
use avatars::{Avatar, Sprites};
use editor::Editor;
use eframe::egui::{self, *};
use network::{Client, Command, Event, Person, Server, Snapshot};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::{path::PathBuf, time::Duration};
use ui_theme as theme;
use web_time::Instant;
use world::RoomMap;
#[derive(Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    name: String,
    space: String,
    address: String,
    port: String,
    color: usize,
    avatar: Avatar,
    media: media::Preferences,
    appearance: theme::Appearance,
    view: view::Preferences,
    browser_host: bool,
    web_port: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            name: "You".into(),
            space: "Our workspace".into(),
            address: "".into(),
            port: "7777".into(),
            color: 0,
            avatar: Avatar::default(),
            media: media::Preferences::default(),
            appearance: Default::default(),
            view: Default::default(),
            browser_host: true,
            web_port: "8080".into(),
        }
    }
}
fn data_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data/settings.json")
}
#[derive(PartialEq, Clone, Copy, Debug)]
enum Dialog {
    Host,
    Join,
    Profile,
    Invite,
    Help,
    Credits,
    Devices,
    Appearance,
    Administration,
    ScreenPicker,
}
struct Commons {
    settings: Settings,
    chrome: chrome::Chrome,
    meeting: meeting::Meeting,
    qa: bool,
    persist: bool,
    server: Option<Server>,
    client: Option<Client>,
    snapshot: Snapshot,
    id: Option<u64>,
    connected: bool,
    connecting: Option<Instant>,
    room: usize,
    position: [f32; 2],
    away: bool,
    dialog: Option<Dialog>,
    password: String,
    admin_password: String,
    is_admin: bool,
    new_room_name: String,
    delete_room_id: Option<u64>,
    chat: String,
    room_chat: bool,
    people_tab: bool,
    search: String,
    notice: String,
    error: String,
    zoom: f32,
    screenshot: Option<PathBuf>,
    frames: usize,
    #[cfg_attr(target_arch = "wasm32", allow(dead_code))]
    screenshot_requested: bool,
    last_move: Instant,
    maps: Vec<RoomMap>,
    world_revision: u64,
    sprites: Sprites,
    remote_motion: remote_motion::Motion,
    editor: Option<Editor>,
    avatar_draft: Avatar,
    profile_name: String,
    preview_direction: u8,
    preview_walking: bool,
    path: VecDeque<[f32; 2]>,
    direction: u8,
    walking: bool,
    camera: Vec2,
    show_grid: bool,
    media: media::Media,
    show_call: bool,
    connected_address: String,
    door_block: Option<(usize, usize, usize)>,
    suspended_editor: Option<(usize, Editor)>,
    suspended_dialog: Option<Dialog>,
}
impl Commons {
    fn new(ctx: &Context, screenshot: Option<PathBuf>) -> Self {
        let mut settings: Settings = storage::read(&data_path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        if !settings.avatar.valid() {
            settings.avatar = Avatar::default();
        }
        theme::apply(ctx, settings.appearance);
        let maps = load_world();
        let position = maps[0].spawn();
        let avatar_draft = settings.avatar.clone();
        let profile_name = settings.name.clone();
        let persist = !cfg!(test) && screenshot.is_none();
        Self {
            persist,
            chrome: Default::default(),
            meeting: Default::default(),
            qa: chrome::qa_enabled(),
            maps,
            world_revision: 0,
            sprites: Sprites::new(ctx),
            remote_motion: Default::default(),
            editor: None,
            avatar_draft,
            profile_name,
            preview_direction: 2,
            preview_walking: true,
            path: VecDeque::new(),
            direction: 2,
            walking: false,
            camera: Vec2::ZERO,
            show_grid: true,
            media: media::Media::new(settings.media.clone()),
            show_call: false,
            connected_address: String::new(),
            door_block: None,
            suspended_editor: None,
            suspended_dialog: None,
            settings,
            server: None,
            client: None,
            snapshot: Snapshot::default(),
            id: None,
            connected: false,
            connecting: None,
            room: 0,
            position,
            away: false,
            dialog: None,
            password: String::new(),
            admin_password: String::new(),
            is_admin: false,
            new_room_name: String::new(),
            delete_room_id: None,
            chat: String::new(),
            room_chat: false,
            people_tab: false,
            search: String::new(),
            notice: String::new(),
            error: String::new(),
            zoom: 1.0,
            screenshot,
            frames: 0,
            screenshot_requested: false,
            last_move: Instant::now(),
        }
    }
    fn save_settings(&self) {
        if !self.persist {
            return;
        }

        let _ = write_json(&data_path(), &self.settings);
    }

    fn send(&self, c: Command) {
        if let Some(client) = &self.client {
            let _ = client.tx.send(c);
        }
    }
    fn disconnect(&mut self) {
        self.is_admin = false;
        self.meeting = Default::default();
        self.admin_password.clear();
        self.remote_motion = Default::default();
        self.media.stop();
        self.client = None;
        self.server = None;
        self.id = None;
        self.connected = false;
        self.connecting = None;
        self.snapshot = Snapshot::default();
        self.maps = load_world();
        self.room = 0;
        self.world_revision = 0;
        self.path.clear();
        self.walking = false;
        self.editor = None;
        self.position = self.maps[self.room].spawn();
        self.notice = "You left the server. Your local preview is still open.".into();
    }
    fn open(&mut self, mut dialog: Dialog) {
        if cfg!(target_arch = "wasm32") && dialog == Dialog::Host {
            dialog = Dialog::Join;
        }
        if self.dialog == Some(Dialog::Devices) && dialog != Dialog::Devices {
            self.media.end_tests();
            if !self.connected {
                self.media.stop();
            }
        }

        if dialog == Dialog::Devices {
            self.media.refresh_devices();
        }
        if dialog == Dialog::Profile {
            self.avatar_draft = self.settings.avatar.clone();
            self.profile_name = self.settings.name.clone();
        }
        self.dialog = Some(dialog);
        self.error.clear();
        self.password.clear();
    }
    fn connect(&mut self, address: String) {
        self.remote_motion = Default::default();
        self.media.stop();
        self.error.clear();
        self.snapshot = Snapshot::default();
        self.world_revision = 0;
        self.room = 0;
        self.connected_address = address.clone();
        self.chat.clear();
        self.client = Some(Client::connect_custom(
            address,
            self.settings.name.trim().to_owned(),
            self.password.clone(),
            self.settings.color,
            self.settings.avatar.clone(),
            self.server
                .as_ref()
                .map(|s| s.editor_key.clone())
                .unwrap_or_default(),
        ));
        self.connecting = Some(Instant::now());
        self.id = None;
        self.connected = false;
        self.save_settings();
    }
    fn poll(&mut self, ctx: &Context) {
        let events: Vec<Event> = self
            .client
            .as_ref()
            .map(|c| c.rx.try_iter().collect())
            .unwrap_or_default();
        for event in events {
            match event {
                Event::AdminStatus(allowed) => {
                    self.is_admin = allowed;
                    self.admin_password.clear();
                    if !allowed {
                        self.editor = None;
                        self.suspended_editor = None;
                    }
                }
                Event::Welcome(id) => {
                    self.id = Some(id);
                    self.connected = true;
                    self.connecting = None;
                    self.dialog = None;
                    self.room = 0;
                    self.position = self.maps[0].spawn();
                    self.path.clear();
                    self.away = false;
                    self.notice = "You're connected. Invite someone to join you.".into();
                    self.error.clear();
                    self.media.change_scope(Some(self.current_scope()));
                    self.media.update_output();
                }
                Event::State(state) => {
                    if state.world_revision != self.world_revision
                        && world::valid_world(&state.rooms)
                    {
                        let previous_id = self.maps.get(self.room).map(|r| r.id);
                        let new_room = state.rooms.iter().position(|r| Some(r.id) == previous_id);
                        if new_room != Some(self.room) {
                            self.path.clear();
                            self.editor = None;
                            self.suspended_editor = None;
                            self.room = new_room.unwrap_or(0);
                            self.position = state
                                .people
                                .iter()
                                .find(|p| Some(p.id) == self.id)
                                .map(|p| [p.x, p.y])
                                .unwrap_or_else(|| state.rooms[self.room].spawn());
                        }
                        self.maps = state.rooms.clone();
                        self.world_revision = state.world_revision;
                        if self.server.is_some() {
                            self.save_world();
                        }
                        if !self.maps[self.room].can_stand(self.position) {
                            self.position = self.maps[self.room].spawn();
                            self.path.clear();
                        }
                        self.send(Command::WorldReady(state.world_revision));
                        self.send_motion();
                    }
                    self.remote_motion
                        .update(&state.people, &self.maps, ctx.input(|i| i.time));
                    self.snapshot = state;
                }
                Event::Notice(message) => {
                    self.notice = message;
                }
                Event::ConversationChanged(_) => {
                    let paused = self.media.pause_sharing();
                    self.sync_conversation();
                    self.send(Command::MediaStatus {
                        mic: false,
                        camera: false,
                    });
                    if paused {
                        self.notice="Conversation changed. Mic and camera paused; turn them on when you're ready.".into();
                    }
                }
                Event::Media {
                    sender,
                    scope,
                    packet,
                } => {
                    self.media.receive(ctx, sender, &scope, &packet);
                }
                Event::Error(error) => {
                    self.is_admin = false;
                    self.error = error;
                    self.connecting = None;
                    self.connected = false;
                }
                Event::Disconnected => {
                    self.is_admin = false;
                    self.media.stop();
                    self.connected = false;
                    self.connecting = None;
                    self.id = None;
                }
            }
        }
        if self
            .connecting
            .is_some_and(|t| t.elapsed() > Duration::from_secs(10))
        {
            self.error = "The server did not answer. Check the address and try again.".into();
            self.client = None;
            self.connecting = None;
        }
    }
    fn people(&self) -> Vec<Person> {
        if self.connected {
            self.snapshot.people.clone()
        } else {
            vec![Person {
                id: 0,
                name: self.settings.name.clone(),
                room: self.room,
                x: self.position[0],
                y: self.position[1],
                color: self.settings.color,
                away: self.away,
                avatar: self.settings.avatar.clone(),
                direction: self.direction,
                walking: self.walking,
                mic: self.media.mic_on,
                camera: self.media.camera_on,
                screen: self.media.screen_on,
            }]
        }
    }
    fn save_world(&self) {
        if !self.persist {
            return;
        }

        if let Err(e) = write_json(&data_dir().join("world.json"), &self.maps) {
            eprintln!("Could not save world: {e}");
        }
    }
    fn send_motion(&mut self) {
        self.sync_conversation();
        self.send(Command::Motion {
            room: self.room,
            x: self.position[0],
            y: self.position[1],
            direction: self.direction,
            walking: self.walking,
        });
    }
    fn go_room(&mut self, room: usize) {
        if self.editor.is_some() {
            self.notice = "Save or cancel your room draft before switching rooms.".into();
            return;
        }
        self.room = room;
        self.position = self.maps[room].spawn();
        self.door_block = Some((
            room,
            (self.position[0] * self.maps[room].width as f32) as usize,
            (self.position[1] * self.maps[room].height as f32) as usize,
        ));
        self.path.clear();
        self.walking = false;
        self.camera = Vec2::ZERO;
        self.zoom = 1.0;
        self.send_motion();
    }
    fn header(&mut self, ctx: &Context) {
        TopBottomPanel::top("top")
            .exact_height(42.0)
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .inner_margin(Margin::symmetric(18, 5))
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.label(
                        RichText::new("c")
                            .color(theme::palette(ctx).accent)
                            .size(25.0),
                    );
                    ui.label(RichText::new("commons").strong().size(18.0));
                    ui.separator();
                    ui.label(
                        RichText::new(if self.connected {
                            &self.snapshot.space
                        } else {
                            &self.settings.space
                        })
                        .size(13.0)
                        .strong(),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(if self.connected {
                                "Connected"
                            } else {
                                "Local preview"
                            })
                            .size(10.0)
                            .color(theme::palette(ctx).muted),
                        );
                        let (r, _) = ui.allocate_exact_size(Vec2::splat(8.0), Sense::hover());
                        ui.painter().circle_filled(
                            r.center(),
                            3.0,
                            if self.connected {
                                Color32::from_rgb(75, 177, 130)
                            } else {
                                theme::palette(ctx).muted
                            },
                        );
                    });
                });
            });
    }
    fn sidebar(&mut self, ctx: &Context) {
        SidePanel::left("navigation")
            .exact_width(224.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).sidebar)
                    .inner_margin(16)
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("SPACE")
                            .size(10.0)
                            .strong()
                            .color(theme::palette(ctx).muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Hide").clicked() {
                            self.settings.view.rooms = false;
                            self.save_settings();
                        }
                    });
                });
                if nav(ui, "Office", !self.people_tab).clicked() {
                    self.people_tab = false;
                }
                if nav(ui, "People", self.people_tab).clicked() {
                    self.people_tab = true;
                }
                ui.add_space(20.0);
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("ROOMS")
                            .size(10.0)
                            .strong()
                            .color(theme::palette(ctx).muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if self
                            .action(
                                ui,
                                "room_edit",
                                icons::Icon::Edit,
                                "Edit",
                                false,
                                self.can_edit_world(),
                            )
                            .on_hover_text("Edit this room")
                            .clicked()
                        {
                            self.begin_edit();
                        }
                    });
                });
                ui.add_space(4.0);
                for (i, name) in self
                    .maps
                    .iter()
                    .map(|m| m.name.clone())
                    .collect::<Vec<_>>()
                    .iter()
                    .enumerate()
                {
                    let count = self.people().iter().filter(|p| p.room == i).count();
                    let r = nav(
                        ui,
                        &format!(
                            "{}   {}    {}",
                            if i == self.room { "•" } else { "○" },
                            name,
                            count
                        ),
                        i == self.room && !self.people_tab,
                    );
                    self.target(["room0", "room1", "room2", "room3"][i], r.rect);
                    if r.clicked() {
                        self.go_room(i);
                        self.people_tab = false;
                    }
                }
            });
    }
    fn chat_panel(&mut self, ctx: &Context) {
        let private_area = self.maps[self.room]
            .area_at(self.position)
            .map(|a| (a.id, a.name.clone()));
        if private_area.is_some() {
            self.room_chat = true;
        }
        SidePanel::right("conversation")
            .exact_width(286.0)
            .resizable(false)
            .frame(
                Frame::new()
                    .fill(theme::palette(ctx).panel)
                    .inner_margin(Margin::same(20))
                    .stroke(Stroke::new(1.0_f32, theme::palette(ctx).border)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Conversation").strong().size(17.0));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.small_button("Hide").clicked(){self.settings.view.conversation=false;self.save_settings();}
                    });
                });
                if let Some((_,name))=&private_area{
                    ui.label(RichText::new(format!("Private · {name}")).strong().color(theme::palette(ui.ctx()).accent));
                    ui.label(RichText::new("Only people here receive this chat, voice and video. Private history stays with its original participants while they remain here.").size(10.0).color(theme::palette(ui.ctx()).muted));
                }else{ui.horizontal(|ui|{ui.selectable_value(&mut self.room_chat,false,"Everyone");ui.selectable_value(&mut self.room_chat,true,"This room");});}
                ui.separator();
                let send_height = 115.0;
                ScrollArea::vertical()
                    .id_salt("chat scroll")
                    .stick_to_bottom(true)
                    .max_height((ui.available_height() - send_height).max(120.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(16.0);
                        if self.snapshot.messages.is_empty() {
                            ui.label(
                                RichText::new("A little hello goes a long way.")
                                    .strong()
                                    .size(14.0),
                            );
                            ui.add_space(3.0);
                            ui.label(RichText::new(if self.connected {
 "Say hi to the people in your space." }
 else {
 "Create or join a server to start a conversation with your people." }
).color(theme::palette(ui.ctx()).muted).size(12.0));
                        }
                        for m in &self.snapshot.messages {
                            if m.area!=private_area.as_ref().map(|(id,_)|*id){continue;}
                            if if self.room_chat {
                                m.room != Some(self.room)
                            } else {
                                m.room.is_some()
                            } {
                                continue;
                            }
                            ui.add_space(8.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(&m.name).strong().size(12.0));
                                ui.label(
                                    RichText::new(format!(
                                        "{:02}:{:02}",
                                        m.time / 3600 % 24,
                                        m.time / 60 % 60
                                    ))
                                    .color(theme::palette(ui.ctx()).muted)
                                    .size(10.0),
                                )
                                .on_hover_text("UTC");
                            });
                            Frame::new()
                                .fill(theme::palette(ui.ctx()).raised)
                                .corner_radius(9)
                                .inner_margin(10)
                                .show(ui, |ui| {
                                    ui.set_width(218.0);
                                    ui.label(&m.text);
                                });
                        }
                    });
                ui.add_space(12.0);
                let r = ui.add_enabled(
                    self.connected,
                    TextEdit::singleline(&mut self.chat)
                        .hint_text("Write a message…")
                        .desired_width(f32::INFINITY)
                        .margin(vec2(10.0, 12.0)),
                );
                self.target("chat_input",r.rect);
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(if self.connected {
                            "Enter to send"
                        } else {
                            "Connect to send messages"
                        })
                        .size(10.0)
                        .color(theme::palette(ui.ctx()).muted),
                    );
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if (ui
                            .add_enabled(
                                self.connected && !self.chat.trim().is_empty(),
                                Button::new("Send  >").fill(theme::palette(ui.ctx()).soft),
                            )
                            .clicked()
                            || enter)
                            && self.connected
                            && !self.chat.trim().is_empty()
                        {
                            self.send(Command::Chat {
                                text: self.chat.clone(),scope:Some(self.current_scope()),
                                room: if self.room_chat {
                                    Some(self.room)
                                } else {
                                    None
                                },
                            });
                            self.chat.clear();
                            r.request_focus();
                        }
                    });
                });
            });
    }
    fn dialogs(&mut self, ctx: &Context) {
        if self.dialog == Some(Dialog::ScreenPicker) {
            self.screen_picker(ctx);
            return;
        }
        if self.dialog == Some(Dialog::Administration) {
            self.administration_dialog(ctx);
            return;
        }
        if self.dialog == Some(Dialog::Devices) {
            self.device_dialog(ctx);
            return;
        }

        if self.dialog == Some(Dialog::Profile) {
            self.avatar_dialog(ctx);
            return;
        }
        if self.dialog == Some(Dialog::Credits) {
            self.credits_dialog(ctx);
            return;
        }
        if self.dialog == Some(Dialog::Appearance) {
            self.appearance_dialog(ctx);
            return;
        }
        let Some(dialog) = self.dialog else {
            return;
        };
        let mut open = true;
        let title = match dialog {
            Dialog::Host => "Create a server",
            Dialog::Join => "Join a server",
            Dialog::Profile => "Your profile",
            Dialog::Invite => "Your server",
            Dialog::Help => "Make yourself at home",
            Dialog::Credits => "Art & credits",
            Dialog::Devices => "Camera & microphone",
            Dialog::Administration => "Administration",
            Dialog::ScreenPicker => "Share screen",
            Dialog::Appearance => "Appearance",
        };
        Window::new(title).open(&mut open).collapsible(false).resizable(false).anchor(Align2::CENTER_CENTER, Vec2::ZERO).default_width(430.0).show(ctx, |ui| {
            ui.add_space(12.0);
            match dialog {
                Dialog::Host | Dialog::Join => {
                    ui.label(RichText::new(if dialog == Dialog::Host {
 "Give your people a place to land." }
 else {
 "Good company is an address away." }
).color(theme::palette(ui.ctx()).muted));
 ui.add_space(9.0);
                    field(ui,"Your name", &mut self.settings.name);
                    if dialog == Dialog::Host {
 field(ui,"Workspace name", &mut self.settings.space);
 field(ui,"Server port", &mut self.settings.port);
 self.host_options(ui);
 }
 else {
 if cfg!(target_arch="wasm32"){ui.label("This browser joins the workspace that served this page.");ui.label(RichText::new(&self.settings.address).monospace());}else{field(ui,"Server address or invite", &mut self.settings.address);}
 ui.label(RichText::new("Paste a browser link (https://…) or a desktop address.").size(11.0).color(theme::palette(ui.ctx()).muted));
 }
                    ui.label(if dialog == Dialog::Host {
 "Password (optional)" }
 else {
 "Password" }
);
 ui.add(TextEdit::singleline(&mut self.password).password(true).desired_width(f32::INFINITY).margin(vec2(10.0,9.0)));
                    ui.add_space(5.0);
 ui.label(RichText::new(if dialog == Dialog::Host {
 "Keep this app open while people visit. Copy the browser link from Servers. For internet access and browser camera/mic, use an HTTPS host. Desktop addresses are intended for trusted networks." }
 else {
 "Ask the host for their address. They need to keep their server running while you are connected." }
).color(theme::palette(ui.ctx()).muted).size(12.0));
                    if !self.error.is_empty() {
 ui.colored_label(Color32::from_rgb(183,65,65), &self.error);
 }
                    ui.add_space(10.0);
                    let busy = self.connecting.is_some();
                    if ui.add_enabled(!busy, Button::new(RichText::new(if busy {
 "Connecting…" }
 else if dialog == Dialog::Host {
 "Create & enter server  >" }
 else if self.connected && self.server.is_some() && !network::Endpoint::parse(&self.settings.address).is_ok_and(|e|e.is_local()&&self.server.as_ref().is_some_and(|s|s.address.port()==e.port||s.web_port()==Some(e.port))) { "Stop hosting & join >" }
 else if self.connected { "Join / switch workspace >" }
 else { "Join workspace >" }
).color(theme::palette(ui.ctx()).primary_text)).fill(theme::palette(ui.ctx()).accent).min_size(vec2(ui.available_width(),42.0))).clicked() {
                        if self.settings.name.trim().is_empty() {
 self.error = "Choose a name so people know who you are.".into();
 }
                        else if dialog == Dialog::Host {
                            if self.connected {
 self.error = "Leave your current server before creating another.".into();
 }
                            else if self.settings.space.trim().is_empty() {
 self.error = "Give your workspace a name.".into();
 }
                            else if let Ok(port) = self.settings.port.parse::<u16>() {
 if port == 0 {
 self.error = "Choose a port between 1 and 65535.".into();
 }
 else {
                                self.client = None;
 self.server = None;
                                match self.create_host(&format!("0.0.0.0:{port}")) {
 Ok(server) => {
 self.server = Some(server);
 self.connect(format!("127.0.0.1:{port}"));
 }
, Err(e) => self.error = e }
                            }
 }
 else {
 self.error = "Use a port number between 1 and 65535.".into();
 }
                        }
                         else {self.join_address(self.settings.address.clone());}
                    }
                    if !cfg!(target_arch="wasm32") && dialog==Dialog::Join && self.server.is_none() && network::Endpoint::parse(&self.settings.address).is_ok_and(|e|e.is_local()) {
                        if ui.button("Host here").clicked(){self.host_here();}
                        ui.label(RichText::new("Create server already connects you. To test another user, open another app instance and join localhost:7777.").size(11.0).color(theme::palette(ui.ctx()).muted));
                    }
if self.connected && ui.button("Leave current server").clicked() {
 self.disconnect();
 }
                }
                Dialog::Invite => {
                    if self.connected {
                        let address = self.server.as_ref().map(|s| format!("{}:{}", network::local_address(), s.address.port())).unwrap_or_else(|| self.settings.address.trim().trim_start_matches("commons://").to_owned());
                        ui.label(RichText::new(&self.snapshot.space).size(23.0).strong());
 ui.label(RichText::new(format!("{} people connected", self.snapshot.people.len())).color(theme::palette(ui.ctx()).muted));
 ui.separator();
 self.browser_invite(ui);
 ui.label("Desktop server address");
                        let invite = if address.starts_with("http://")||address.starts_with("https://"){address.clone()}else{format!("commons://{address}")};
 ui.label(RichText::new(&invite).monospace().color(theme::palette(ui.ctx()).accent));
                        if primary(ui,"Copy invite").clicked() {
 ctx.copy_text(invite);
 self.notice = "Invite copied to your clipboard.".into();
 }
                        ui.label(RichText::new("Browser guests open the browser link; desktop guests can use the server address. Outside your local network, use your public HTTPS address. Share a password separately if you set one.").color(theme::palette(ui.ctx()).muted).size(12.0));
                        ui.add_space(12.0);
 if ui.button(if self.server.is_some() {
 "Stop server & leave" }
 else {
 "Leave server" }
).clicked() {
 self.disconnect();
 self.dialog = None;
 }
                    }
 else {
 ui.label("Create or join a server to invite people.");
 if primary(ui,"Create server").clicked() {
 self.open(Dialog::Host);
 }
 }
                }
                Dialog::ScreenPicker | Dialog::Administration | Dialog::Profile | Dialog::Credits | Dialog::Devices | Dialog::Appearance => {
}
                Dialog::Help => {
                    for (number,title,text) in [("01","Make a space","Choose Create server, name your workspace, and enter. Your computer becomes the host."),("02","Bring your people","Copy an invite from Servers. Other people choose Join server and paste the address."),("03","Feel at home","Click the floor or use arrow keys / WASD to move. Choose a room on the left. Chat with everyone or just your room.")] {
                        ui.label(RichText::new(format!("{number}   {title}")).strong().color(theme::palette(ui.ctx()).accent));
 ui.label(text);
 ui.add_space(9.0);
                    }
                    ui.separator();
ui.label("Choose Edit room to paint floors, place props, resize the map, or apply a theme. Save your design to share it with connected guests. The host controls shared rooms.");
ui.label("Open your profile to change skin, expression, hair, clothes, and accessories, even while connected.");
ui.label("In Edit room, choose Areas & doors to place private conversation rectangles and doors to other rooms. Walk onto a door to use it. Private conversations are filtered by the server; crossing a conversation boundary pauses mic and camera sharing.");
                    ui.label(RichText::new("Alt + scroll zooms around your pointer. Use Audio / video settings to choose devices and preview them locally. Mic and camera sharing stay off until enabled in a connected conversation.").color(theme::palette(ui.ctx()).muted).size(12.0));
if ui.link("Art sources & credits").clicked(){
self.dialog=Some(Dialog::Credits);
}
                }
            }
            ui.add_space(10.0);
        }
);
        if !open {
            self.dialog = None;
        }
    }
}
impl eframe::App for Commons {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        self.draw_workspace(ctx);
        self.poll_media(ctx);

        #[cfg(target_arch = "wasm32")]
        browser::publish(self);
        self.frames += 1;
        #[cfg(not(target_arch = "wasm32"))]
        if self.screenshot.is_some() {
            if self.frames >= 120 && !self.screenshot_requested {
                ctx.send_viewport_cmd(ViewportCommand::Screenshot(UserData::default()));
                self.screenshot_requested = true;
            }
            let pictures = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| {
                        if let egui::Event::Screenshot { image, .. } = e {
                            Some(image.clone())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
            });
            for picture in pictures {
                if let Some(path) = &self.screenshot {
                    if let Some(parent) = path.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    let bytes: Vec<u8> = picture.pixels.iter().flat_map(|p| p.to_array()).collect();
                    match image::save_buffer(
                        path,
                        &bytes,
                        picture.width() as u32,
                        picture.height() as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(_) => {
                            println!("Saved screenshot: {}", path.display());
                            ctx.send_viewport_cmd(ViewportCommand::Close);
                        }
                        Err(e) => {
                            eprintln!("Screenshot failed: {e}");
                            std::process::exit(1);
                        }
                    }
                }
            }
        }
        ctx.request_repaint_after(Duration::from_millis(16));
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.media.stop();
        if self.screenshot.is_none() {
            self.save_settings();
        }
        self.client = None;
        self.server = None;
    }
}
fn primary(ui: &mut Ui, text: &str) -> Response {
    ui.add(
        Button::new(RichText::new(text).color(theme::palette(ui.ctx()).primary_text))
            .fill(theme::palette(ui.ctx()).accent)
            .corner_radius(8),
    )
}
fn nav(ui: &mut Ui, text: &str, active: bool) -> Response {
    ui.add(
        Button::new(RichText::new(text).size(13.0).color(if active {
            theme::palette(ui.ctx()).accent
        } else {
            theme::palette(ui.ctx()).muted
        }))
        .fill(if active {
            theme::palette(ui.ctx()).soft
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .min_size(vec2(ui.available_width(), 39.0))
        .corner_radius(8),
    )
}
fn field(ui: &mut Ui, label: &str, value: &mut String) {
    ui.label(label);
    ui.add(
        TextEdit::singleline(value)
            .desired_width(f32::INFINITY)
            .margin(vec2(10.0, 9.0)),
    );
}
#[cfg(not(target_arch = "wasm32"))]
pub fn run_native() -> eframe::Result {
    if std::env::args().any(|s| s == "--serve-web") {
        if let Err(e) = hosting::serve_cli() {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let mut args = std::env::args().skip(1);
    let mut screenshot = None;
    let mut view = String::new();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                screenshot = Some(PathBuf::from(
                    args.next().expect("--screenshot needs a PNG file"),
                ));
            }
            "--camera-self-test" => {
                let result =
                    camera_backend::self_test().unwrap_or_else(|e| serde_json::json!({"error":e}));
                println!("{result}");
                return Ok(());
            }
            "--camera-diagnostics" => {
                let report=camera_backend::discover().map(|r|serde_json::json!({"status":r.status(),"camera_count":r.devices.len(),"names":r.devices.iter().map(|d|d.name.clone()).collect::<Vec<_>>(),"error":r.error})).unwrap_or_else(|e|serde_json::json!({"error":e}));
                println!("{report}");
                return Ok(());
            }
            "--view" => {
                view = args.next().unwrap_or_default();
            }
            "--help" | "-h" => {
                println!("Commons — shared virtual workspace\n\n--screenshot <file.png>  Save the window after 120 frames and exit\n--view editor|avatar|credits  Open a particular workspace view\n--serve-web  Host browser guests without opening a window\n--http-bind 0.0.0.0:8080  Browser listener\n--tcp-bind 127.0.0.1:7777  Desktop listener\n--name <name>  Workspace name; password from COMMONS_PASSWORD");
                return Ok(());
            }
            _ => {}
        }
    }
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_min_inner_size([1100.0, 720.0]),
        renderer: eframe::Renderer::Glow,
        dithering: false,
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "Commons — Your shared workspace",
        options,
        Box::new(move |cc| {
            let mut app = Commons::new(&cc.egui_ctx, screenshot);
            match view.as_str() {
                "profile-menu" => {
                    app.settings.view.map_only = false;
                    app.chrome.profile_open = true;
                }
                "private" => {
                    app.settings.view.map_only = false;
                    if let Some(area) = app.maps[0].areas.first() {
                        app.position = [
                            (area.x as f32 + area.width as f32 / 2.0) / app.maps[0].width as f32,
                            (area.y as f32 + area.height as f32 / 2.0) / app.maps[0].height as f32,
                        ];
                        app.sync_conversation();
                    }
                }
                "editor" => app.begin_edit(),
                "host" => app.open(Dialog::Host),
                "avatar" => app.open(Dialog::Profile),
                "credits" => app.open(Dialog::Credits),
                "theme" => app.open(Dialog::Appearance),
                "dark" => {
                    app.settings.appearance.mode = theme::Mode::Dark;
                }
                "map" => {
                    app.settings.view.map_only = false;
                    app.toggle_map_only();
                }
                "female" => {
                    app.open(Dialog::Profile);
                    app.avatar_draft.body = 1;
                    app.avatar_draft.hair = 7;
                    app.avatar_draft.shirt = 3;
                    app.avatar_draft.bottom = 2;
                    app.avatar_draft.accessory = 5;
                }
                "devices" => app.open(Dialog::Devices),
                "spaces" | "doors" => {
                    app.begin_edit();
                    if let Some(e) = &mut app.editor {
                        e.category = "Areas & doors".into();
                        e.tool = if view == "doors" {
                            editor::Tool::Door
                        } else {
                            editor::Tool::Area
                        };
                    }
                }
                _ => {}
            }
            Ok(Box::new(app))
        }),
    )
}
fn data_dir() -> PathBuf {
    storage::data_dir()
}
fn write_json<T: Serialize>(path: &std::path::Path, value: &T) -> Result<(), String> {
    storage::write(path, value)
}
fn load_world() -> Vec<RoomMap> {
    let mut maps = storage::read(&data_dir().join("world.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<RoomMap>>(&b).ok())
        .filter(|maps| world::valid_world(maps))
        .unwrap_or_else(world::initial_world);
    world::normalize_world(&mut maps);
    maps
}

#[cfg(test)]
mod ui_tests {
    use super::*;
    #[test]
    fn leaving_a_remote_map_resets_location_and_administrator_session() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.room = 127;
        app.is_admin = true;
        app.admin_password = "temporary".into();
        app.meeting.active = true;
        app.disconnect();
        assert_eq!(app.room, 0);
        assert_eq!(app.position, app.maps[0].spawn());
        assert!(!app.is_admin);
        assert!(app.admin_password.is_empty());
        assert!(!app.meeting.active);
    }
    fn test_app(ctx: &Context) -> Commons {
        let mut app = Commons::new(ctx, None);
        app.maps = world::default_world();
        app.position = app.maps[0].spawn();
        app.settings.view = Default::default();
        app.settings.appearance = Default::default();
        theme::apply(ctx, app.settings.appearance);
        app
    }

    fn draw(
        ctx: &Context,
        app: &mut Commons,
        events: Vec<egui::Event>,
        time: &mut f64,
    ) -> FullOutput {
        *time += 0.02;
        ctx.run(
            RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440.0, 900.0))),
                time: Some(*time),
                events,
                ..Default::default()
            },
            |ctx| {
                app.draw_workspace(ctx);
            },
        )
    }
    fn text_rect(output: &FullOutput, text: &str) -> Rect {
        fn find(shape: &epaint::Shape, text: &str) -> Option<Rect> {
            match shape {
                epaint::Shape::Text(t) if t.galley.text() == text => {
                    Some(Rect::from_min_size(t.pos, t.galley.size()))
                }
                epaint::Shape::Vec(v) => v.iter().find_map(|s| find(s, text)),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .rev()
            .find_map(|s| find(&s.shape, text))
            .unwrap_or_else(|| panic!("Could not find UI text: {text}"))
    }
    fn click(ctx: &Context, app: &mut Commons, point: Pos2, time: &mut f64) -> FullOutput {
        let _ = draw(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::default(),
                },
            ],
            time,
        );
        draw(
            ctx,
            app,
            vec![egui::Event::PointerButton {
                pos: point,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Modifiers::default(),
            }],
            time,
        )
    }
    fn rendered_map(output: &FullOutput) -> Rect {
        fn find(s: &epaint::Shape) -> Option<Rect> {
            match s {
                epaint::Shape::Rect(r) if r.fill == Color32::from_rgb(48, 45, 62) => Some(r.rect),
                epaint::Shape::Vec(v) => v.iter().find_map(find),
                _ => None,
            }
        }
        output
            .shapes
            .iter()
            .find_map(|s| find(&s.shape))
            .expect("map should render")
    }
    fn press_control(ctx: &Context, app: &mut Commons, id: &str, time: &mut f64) -> FullOutput {
        let _ = draw(ctx, app, vec![], time);
        let _ = draw(ctx, app, vec![], time);
        let point = app
            .chrome
            .targets
            .get(id)
            .unwrap_or_else(|| panic!("Missing control {id}: {:?}", app.chrome.targets.keys()))
            .center();
        click(ctx, app, point, time)
    }
    #[test]
    fn profile_menu_groups_workspace_controls_and_status_with_a_clear_map() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let map = app.chrome.map_rect.unwrap();
        assert!(map.top() < 65.0);
        assert!(map.height() > 770.0);
        let profile = app.chrome.targets["profile"];
        assert!(profile.left() < 24.0 && profile.bottom() > 850.0);
        assert!(!app.chrome.targets.contains_key("theme"));
        press_control(&ctx, &mut app, "profile", &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        for id in [
            "avatar",
            "theme",
            "view_menu",
            "create_server",
            "join_server",
        ] {
            assert!(app.chrome.targets.contains_key(id), "{id}");
        }
        press_control(&ctx, &mut app, "status_menu", &mut time);
        press_control(&ctx, &mut app, "status_away", &mut time);
        assert!(app.away);
        press_control(&ctx, &mut app, "theme", &mut time);
        assert_eq!(app.dialog, Some(Dialog::Appearance));
        assert!(!app.chrome.profile_open);
    }
    #[test]
    fn avatar_remains_interactive_after_using_the_status_submenu() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        let mut time = 0.0;
        press_control(&ctx, &mut app, "profile", &mut time);
        press_control(&ctx, &mut app, "status_menu", &mut time);
        press_control(&ctx, &mut app, "status_away", &mut time);
        press_control(&ctx, &mut app, "avatar", &mut time);
        assert_eq!(app.dialog, Some(Dialog::Profile));
        press_control(&ctx, &mut app, "body_feminine", &mut time);
        assert_eq!(app.avatar_draft.body, 1);
        press_control(&ctx, &mut app, "avatar_save", &mut time);
        assert_eq!(app.settings.avatar.body, 1);
        assert!(app.dialog.is_none());
    }
    #[test]
    fn camera_follows_keyboard_and_click_walking_on_a_zoomed_map() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps[0] = RoomMap::blank("Camera test", 40, 40, 0);
        app.position = [0.5, 0.5];
        app.zoom = 4.0;
        let mut time = 0.0;
        let assert_visible = |app: &Commons| {
            let map = app.chrome.map_rect.unwrap();
            let world = app.chrome.world_rect.unwrap();
            let feet = world.min
                + vec2(
                    app.position[0] * world.width(),
                    app.position[1] * world.height(),
                );
            assert!(
                map.shrink(24.0).contains(feet),
                "Character left viewport: {feet:?} in {map:?}"
            );
            assert!(map.contains(feet - vec2(0.0, world.height() / 40.0 * 2.0)));
        };
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::ArrowRight,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            &mut time,
        );
        for _ in 0..150 {
            let _ = draw(&ctx, &mut app, vec![], &mut time);
            assert_visible(&app);
        }
        let _ = draw(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::ArrowRight,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            &mut time,
        );
        assert!(app.position[0] > 0.9);
        assert!(app.camera.x < -400.0);
        let camera = app.camera;
        let map = app.chrome.map_rect.unwrap();
        let world = app.chrome.world_rect.unwrap();
        let target = pos2(
            map.left() + 40.0,
            world.top() + app.position[1] * world.height(),
        );
        click(&ctx, &mut app, target, &mut time);
        assert!(!app.path.is_empty());
        for _ in 0..120 {
            let _ = draw(&ctx, &mut app, vec![], &mut time);
            assert_visible(&app);
        }
        assert!(app.path.is_empty());
        assert!(app.camera.x > camera.x + 50.0);
    }
    #[test]
    fn idle_pan_is_preserved_until_walking_resumes() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps[0] = RoomMap::blank("Camera test", 40, 40, 0);
        app.position = [0.5, 0.5];
        app.zoom = 3.0;
        app.camera = vec2(900.0, -900.0);
        let mut time = 0.0;
        for _ in 0..3 {
            let _ = draw(&ctx, &mut app, vec![], &mut time);
        }
        assert_eq!(app.camera, vec2(900.0, -900.0));
        let _ = draw(
            &ctx,
            &mut app,
            vec![egui::Event::Key {
                key: Key::ArrowDown,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Modifiers::NONE,
            }],
            &mut time,
        );
        assert!(app.camera.x < 900.0 && app.camera.y > -900.0);
        app.begin_edit();
        app.zoom = 3.0;
        app.camera = vec2(900.0, -900.0);
        let position = app.position;
        for _ in 0..3 {
            let _ = draw(&ctx, &mut app, vec![], &mut time);
        }
        assert_eq!(app.position, position);
        assert_eq!(app.camera, vec2(900.0, -900.0));
    }
    #[test]
    fn compact_zoom_controls_do_not_click_through_to_the_map() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        let mut time = 0.0;
        let position = app.position;
        press_control(&ctx, &mut app, "zoom_in", &mut time);
        assert!(app.zoom > 1.0);
        assert!(app.path.is_empty());
        assert_eq!(app.position, position);
        let map = app.chrome.map_rect.unwrap();
        let controls = app.chrome.targets["zoom_controls"];
        assert!(map.contains_rect(controls));
        assert!(controls.width() < 290.0 && controls.height() < 60.0);
        app.camera = vec2(80.0, 90.0);
        press_control(&ctx, &mut app, "zoom_fit", &mut time);
        assert_eq!(app.zoom, 1.0);
        assert_eq!(app.camera, Vec2::ZERO);
        assert!(app.path.is_empty());
    }
    #[test]
    fn private_call_strip_automatically_opens_is_fixed_and_reopens_after_reentry() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps = world::default_world();
        let area = app.maps[0].areas[0].clone();
        let private = [
            (area.x as f32 + 1.0) / app.maps[0].width as f32,
            (area.y as f32 + 1.0) / app.maps[0].height as f32,
        ];
        app.position = private;
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        assert!(app.show_call);
        assert!(!app.media.mic_on && !app.media.camera_on);
        let map = app.chrome.map_rect.unwrap();
        let strip = app.chrome.targets["call_strip"];
        assert!((strip.center().x - map.center().x).abs() < 2.0);
        assert!(strip.top() >= map.top() && strip.top() < map.top() + 25.0);
        let start = strip.left_top() + vec2(20.0, 12.0);
        let _ = draw(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
            &mut time,
        );
        let end = start + vec2(80.0, 90.0);
        let _ = draw(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(end)],
            &mut time,
        );
        let _ = draw(
            &ctx,
            &mut app,
            vec![egui::Event::PointerButton {
                pos: end,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
            &mut time,
        );
        assert_eq!(app.chrome.targets["call_strip"].min, strip.min);
        press_control(&ctx, &mut app, "call_collapse", &mut time);
        assert!(app.chrome.call_collapsed);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        assert!(app.chrome.targets["call_strip"].height() < 80.0);
        app.position = app.maps[0].spawn();
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        assert!(!app.show_call);
        app.position = private;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        assert!(app.show_call && !app.chrome.call_collapsed);
    }
    #[test]
    fn map_only_restores_draft_and_panels_without_blocking_navigation() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps = world::default_world();
        app.begin_edit();
        app.editor.as_mut().unwrap().map.name = "Unfinished draft".into();
        app.open(Dialog::Profile);
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let event = |key| egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::default(),
        };
        let _ = draw(&ctx, &mut app, vec![event(Key::F10)], &mut time);
        assert!(app.settings.view.map_only);
        assert!(app.editor.is_none());
        assert!(app.dialog.is_none());
        let output = draw(&ctx, &mut app, vec![], &mut time);
        text_rect(&output, "Show controls · Esc");
        assert!(rendered_map(&output).width() > 1400.0);
        assert!(rendered_map(&output).height() > 880.0);
        let _ = draw(&ctx, &mut app, vec![event(Key::Escape)], &mut time);
        assert!(!app.settings.view.map_only);
        assert_eq!(app.editor.as_ref().unwrap().map.name, "Unfinished draft");
        assert_eq!(app.dialog, Some(Dialog::Profile));
        app.dialog = None;
        app.editor = None;
        app.settings.view.rooms = false;
        app.settings.view.conversation = false;
        app.settings.view.calls = false;
        let output = draw(&ctx, &mut app, vec![], &mut time);
        assert!(rendered_map(&output).width() > 1380.0);
        app.settings.view = view::Preferences::default();
        let output = draw(&ctx, &mut app, vec![], &mut time);
        assert!(rendered_map(&output).width() < 1000.0);
    }
    #[test]
    fn appearance_controls_apply_dark_mode_and_new_accent() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.open(Dialog::Appearance);
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let output = draw(&ctx, &mut app, vec![], &mut time);
        let dark = text_rect(&output, "Dark").center();
        let output = click(&ctx, &mut app, dark, &mut time);
        assert_eq!(app.settings.appearance.mode, theme::Mode::Dark);
        assert!(ctx.style().visuals.dark_mode);
        let before = theme::palette(&ctx).accent;
        let ocean = text_rect(&output, "Ocean").center();
        let _ = click(&ctx, &mut app, ocean, &mut time);
        assert_eq!(app.settings.appearance.scheme, theme::Scheme::Ocean);
        assert_ne!(before, theme::palette(&ctx).accent);
    }
    #[test]
    fn real_pointer_input_opens_paints_and_cancels_editor_and_changes_avatar_preview() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps = world::default_world();
        app.position = app.maps[0].spawn();
        let original = app.maps.clone();
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let button = app.chrome.targets["room_edit"].center();
        let _ = click(&ctx, &mut app, button, &mut time);
        assert!(app.editor.is_some());
        let output = click(&ctx, &mut app, pos2(500.0, 450.0), &mut time);
        assert_ne!(app.editor.as_ref().unwrap().map, original[0]);
        let cancel = text_rect(&output, "Cancel").center();
        let _ = click(&ctx, &mut app, cancel, &mut time);
        assert!(app.editor.is_none());
        assert_eq!(app.maps, original);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let profile = app.chrome.targets["profile"].center();
        let _ = click(&ctx, &mut app, profile, &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let avatar = app.chrome.targets["avatar"].center();
        let _ = click(&ctx, &mut app, avatar, &mut time);
        assert_eq!(app.dialog, Some(Dialog::Profile));
        let output = draw(&ctx, &mut app, vec![], &mut time);
        let skin = text_rect(&output, "Skin");
        let point = pos2(skin.left() + 43.5, skin.bottom() + 12.0 + 14.5);
        let previous = app.settings.avatar.clone();
        let _ = click(&ctx, &mut app, point, &mut time);
        assert_eq!(app.avatar_draft.skin, 1);
        assert_eq!(app.settings.avatar, previous);
    }
    #[test]
    fn room_designs_roundtrip_through_atomic_storage() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join(format!("target/test-design-{}.json", rand::random::<u64>()));
        let mut map = world::preset(0, "Saved world", 32, 24);
        map.stamp(46, 7, 9, false);
        write_json(&path, &map).unwrap();
        let reloaded: RoomMap = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        assert_eq!(map, reloaded);
        assert!(reloaded.validate());
        std::fs::remove_file(path).unwrap();
    }
    #[test]
    fn hosting_then_joining_your_own_localhost_keeps_the_connection() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.media.listening = false;
        let server = Server::start_with_rooms(
            "127.0.0.1:0",
            "Own server".into(),
            "".into(),
            world::default_world(),
        )
        .unwrap();
        let port = server.address.port();
        app.server = Some(server);
        app.connect(format!("localhost:{port}"));
        let end = Instant::now() + Duration::from_secs(3);
        while !app.connected && Instant::now() < end {
            app.poll(&ctx);
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(app.connected, "{}", app.error);
        let id = app.id;
        app.dialog = Some(Dialog::Join);
        app.join_address(format!("commons://localhost:{port}/"));
        assert_eq!(app.id, id);
        assert!(app.server.is_some());
        assert!(app.connected);
        assert!(app.error.is_empty());
        assert!(app.notice.contains("already connected"));
        let other = Client::connect(
            format!("localhost:{port}"),
            "Second instance".into(),
            "".into(),
            0,
        );
        let event = other.rx.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(matches!(event, Event::Welcome(_)));
    }
    #[test]
    fn doors_transition_to_their_destination_without_bouncing_back() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        app.maps = world::default_world();
        let door = app.maps[0].doors[0].clone();
        app.position = [
            (door.x as f32 + 0.5) / app.maps[0].width as f32,
            (door.y as f32 + 0.5) / app.maps[0].height as f32,
        ];
        app.try_door();
        assert_eq!(app.room, door.destination);
        assert!(app.maps[app.room].can_stand(app.position));
        let position = app.position;
        app.try_door();
        assert_eq!(app.position, position);
    }
    #[test]
    fn alt_mouse_wheel_zooms_and_does_not_drift_after_release() {
        let ctx = Context::default();
        let mut app = test_app(&ctx);
        let mut time = 0.0;
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let _ = draw(&ctx, &mut app, vec![], &mut time);
        let alt = Modifiers {
            alt: true,
            ..Default::default()
        };
        let _ = draw(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(pos2(600.0, 450.0)),
                egui::Event::MouseWheel {
                    unit: egui::MouseWheelUnit::Point,
                    delta: vec2(0.0, 120.0),
                    modifiers: alt,
                },
            ],
            &mut time,
        );
        assert!(app.zoom > 1.0);
        let camera = app.camera;
        for _ in 0..20 {
            let _ = draw(&ctx, &mut app, vec![], &mut time);
        }
        assert!((app.camera - camera).length() < 0.001);
    }
}
