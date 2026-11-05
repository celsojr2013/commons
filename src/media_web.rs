pub use crate::media_types::*;
use crate::{
    network::{MediaKind, MediaPacket},
    world::TalkScope,
};
use eframe::egui::{Context, TextureOptions};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_time::{Duration, Instant};
#[wasm_bindgen(module = "/web/media.js")]
extern "C" {
    pub type BrowserMedia;
    #[wasm_bindgen(constructor)]
    fn new() -> BrowserMedia;
    #[wasm_bindgen(method)]
    fn input(this: &BrowserMedia, active: bool, device: &str, generation: u32);
    #[wasm_bindgen(method)]
    fn camera(
        this: &BrowserMedia,
        active: bool,
        device: &str,
        generation: u32,
        width: u32,
        height: u32,
        fps: u32,
        quality: u32,
    );
    #[wasm_bindgen(method)]
    fn screen(this: &BrowserMedia, active: bool, generation: u32);
    #[wasm_bindgen(method)]
    fn output(this: &BrowserMedia, active: bool, device: &str, volume: f32);
    #[wasm_bindgen(method)]
    fn permission(this: &BrowserMedia);
    #[wasm_bindgen(method)]
    fn refresh(this: &BrowserMedia);
    #[wasm_bindgen(method)]
    fn poll(this: &BrowserMedia) -> String;
    #[wasm_bindgen(method)]
    fn play(this: &BrowserMedia, sender: f64, data: &str);
    #[wasm_bindgen(method)]
    fn clear(this: &BrowserMedia);
    #[wasm_bindgen(method)]
    fn retain(this: &BrowserMedia, ids: &str);
    #[wasm_bindgen(method)]
    fn tone(this: &BrowserMedia);
    #[wasm_bindgen(method)]
    fn stop(this: &BrowserMedia);
}
#[derive(serde::Deserialize, Default)]
struct Report {
    #[serde(default)]
    events: Vec<Frame>,
    #[serde(default)]
    devices: Vec<Device>,
    #[serde(default)]
    status: String,
    #[serde(default)]
    level: f32,
}
#[derive(serde::Deserialize)]
struct Device {
    id: String,
    name: String,
    kind: String,
}
#[derive(serde::Deserialize)]
struct Frame {
    kind: String,
    #[serde(default)]
    generation: u32,
    #[serde(default)]
    data: String,
    #[serde(default)]
    text: String,
    #[serde(default)]
    source: String,
}
pub struct Media {
    pub preferences: Preferences,
    pub devices: Devices,
    pub error: String,
    pub camera_status: String,
    pub mic_on: bool,
    pub camera_on: bool,
    pub screen_on: bool,
    pub screen_ready: bool,
    pub screen_sources: Vec<ScreenSource>,
    pub screen_source: String,
    pub local_screen: Option<VideoView>,
    pub screens: HashMap<u64, VideoView>,
    screen_generation: u32,
    pub mic_test: bool,
    pub preview: bool,
    pub listening: bool,
    pub mic_ready: bool,
    pub camera_ready: bool,
    pub local_video: Option<VideoView>,
    pub remote: HashMap<u64, VideoView>,
    scope: Option<TalkScope>,
    bridge: BrowserMedia,
    input_generation: u32,
    camera_generation: u32,
    level: f32,
}
impl Media {
    pub fn new(preferences: Preferences) -> Self {
        Self {
            preferences,
            devices: Devices::default(),
            error: String::new(),
            camera_status: "Choose a local device test to grant browser permissions.".into(),
            mic_on: false,
            camera_on: false,
            screen_on: false,
            screen_ready: false,
            screen_sources: vec![],
            screen_source: String::new(),
            local_screen: None,
            screens: HashMap::new(),
            screen_generation: 0,
            mic_test: false,
            preview: false,
            listening: true,
            mic_ready: false,
            camera_ready: false,
            local_video: None,
            remote: HashMap::new(),
            scope: None,
            bridge: BrowserMedia::new(),
            input_generation: 0,
            camera_generation: 0,
            level: 0.0,
        }
    }
    pub fn refresh_devices(&mut self) {
        self.bridge.refresh();
    }
    pub fn request_camera_access(&mut self) {
        self.bridge.permission();
    }
    pub fn update_input(&mut self) {
        self.input_generation = self.input_generation.wrapping_add(1);
        self.mic_ready = false;
        self.bridge.input(
            self.mic_on || self.mic_test,
            &self.preferences.microphone,
            self.input_generation,
        );
    }
    pub fn update_camera(&mut self) {
        self.camera_generation = self.camera_generation.wrapping_add(1);
        self.camera_ready = false;
        self.local_video = None;
        let (width, height, fps, quality) = self.preferences.quality.profile();
        self.bridge.camera(
            self.camera_on || self.preview,
            &self.preferences.camera,
            self.camera_generation,
            width,
            height,
            fps,
            quality as u32,
        );
    }
    pub fn refresh_screens(&mut self) {
        self.screen_sources.clear();
    }
    pub fn start_screen(&mut self) {
        self.screen_generation = self.screen_generation.wrapping_add(1);
        self.screen_on = true;
        self.screen_ready = false;
        self.bridge.screen(true, self.screen_generation);
    }
    pub fn stop_screen(&mut self) {
        self.screen_generation = self.screen_generation.wrapping_add(1);
        self.screen_on = false;
        self.screen_ready = false;
        self.local_screen = None;
        self.bridge.screen(false, self.screen_generation);
    }
    pub fn update_output(&mut self) {
        self.bridge.output(
            self.listening,
            &self.preferences.speaker,
            self.preferences.volume,
        );
    }
    pub fn set_volume(&self) {
        self.bridge.output(
            self.listening,
            &self.preferences.speaker,
            self.preferences.volume,
        );
    }
    pub fn test_speaker(&mut self) {
        self.listening = true;
        self.update_output();
        self.bridge.tone();
    }
    pub fn level(&self) -> f32 {
        self.level
    }
    pub fn scope(&self) -> Option<&TalkScope> {
        self.scope.as_ref()
    }
    pub fn pause_sharing(&mut self) -> bool {
        let was = self.mic_on || self.camera_on || self.screen_on;
        self.stop_screen();
        self.mic_on = false;
        self.camera_on = false;
        self.update_input();
        self.update_camera();
        self.clear_remote();
        was
    }
    pub fn change_scope(&mut self, scope: Option<TalkScope>) -> bool {
        if self.scope == scope {
            return false;
        }
        self.scope = scope;
        self.pause_sharing()
    }
    pub fn clear_remote(&mut self) {
        self.remote.clear();
        self.screens.clear();
        self.bridge.clear();
    }
    pub fn end_tests(&mut self) {
        if self.mic_test {
            self.mic_test = false;
            self.update_input();
        }
        if self.preview {
            self.preview = false;
            if !self.camera_on {
                self.update_camera();
            }
        }
    }
    pub fn stop(&mut self) {
        self.stop_screen();
        self.mic_on = false;
        self.camera_on = false;
        self.mic_test = false;
        self.preview = false;
        self.input_generation = self.input_generation.wrapping_add(1);
        self.camera_generation = self.camera_generation.wrapping_add(1);
        self.bridge.stop();
        self.local_video = None;
        self.clear_remote();
        self.scope = None;
    }
    pub fn poll(&mut self, ctx: &Context, connected: bool) -> Vec<MediaPacket> {
        let report: Report = serde_json::from_str(&self.bridge.poll()).unwrap_or_default();
        self.level = report.level;
        self.camera_status = report.status;
        self.devices.microphones = report
            .devices
            .iter()
            .filter(|d| d.kind == "audioinput")
            .map(|d| d.name.clone())
            .collect();
        self.devices.speakers = report
            .devices
            .iter()
            .filter(|d| d.kind == "audiooutput")
            .map(|d| d.name.clone())
            .collect();
        self.devices.cameras = report
            .devices
            .into_iter()
            .filter(|d| d.kind == "videoinput")
            .map(|d| (d.id, d.name))
            .collect();
        let mut packets = vec![];
        for event in report.events {
            match event.kind.as_str() {
                "audio" if event.generation == self.input_generation => {
                    self.mic_ready = true;
                    if self.mic_on && connected {
                        packets.push(MediaPacket {
                            kind: MediaKind::Audio,
                            data: event.data,
                            scope: None,
                        });
                    }
                }
                "video" if event.generation == self.camera_generation => {
                    self.camera_ready = true;
                    let packet = MediaPacket {
                        kind: MediaKind::Video,
                        data: event.data,
                        scope: None,
                    };
                    if let Ok(bytes) = packet.bytes() {
                        if let Ok(image) = decode_video(&bytes, self.preferences.mirror) {
                            let texture =
                                ctx.load_texture("local camera", image, TextureOptions::LINEAR);
                            self.local_video = Some(VideoView {
                                texture,
                                received: Instant::now(),
                            });
                        }
                    }
                    if self.camera_on && connected {
                        packets.push(packet);
                    }
                }
                "screen" if event.generation == self.screen_generation && self.screen_on => {
                    self.screen_ready = true;
                    let packet = MediaPacket {
                        kind: MediaKind::Screen,
                        data: event.data,
                        scope: None,
                    };
                    if let Ok(bytes) = packet.bytes() {
                        if let Ok(image) = decode_video(&bytes, false) {
                            let texture =
                                ctx.load_texture("local screen", image, TextureOptions::LINEAR);
                            self.local_screen = Some(VideoView {
                                texture,
                                received: Instant::now(),
                            });
                        }
                    }
                    if connected {
                        packets.push(packet);
                    }
                }
                "screen_end" if event.generation == self.screen_generation => self.stop_screen(),
                "error" => {
                    let current = match event.source.as_str() {
                        "camera" => event.generation == self.camera_generation,
                        "screen" => event.generation == self.screen_generation,
                        "microphone" => event.generation == self.input_generation,
                        _ => true,
                    };
                    if current {
                        self.error = event.text;
                        match event.source.as_str() {
                            "screen" => self.stop_screen(),
                            "camera" => {
                                self.camera_on = false;
                                self.preview = false;
                                self.update_camera();
                            }
                            "microphone" => {
                                self.mic_on = false;
                                self.mic_test = false;
                                self.update_input();
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
        packets
    }
    pub fn receive(&mut self, ctx: &Context, sender: u64, scope: &TalkScope, packet: &MediaPacket) {
        if self.scope.as_ref() != Some(scope) {
            return;
        }
        match packet.kind {
            MediaKind::Audio => {
                if self.listening {
                    self.bridge.play(sender as f64, &packet.data);
                }
            }
            MediaKind::Video | MediaKind::Screen => {
                if let Ok(bytes) = packet.bytes() {
                    if let Ok(image) = decode_video(&bytes, false) {
                        let texture = ctx.load_texture(
                            format!("camera {sender}"),
                            image,
                            TextureOptions::LINEAR,
                        );
                        let images = if packet.kind == MediaKind::Screen {
                            &mut self.screens
                        } else {
                            &mut self.remote
                        };
                        images.insert(
                            sender,
                            VideoView {
                                texture,
                                received: Instant::now(),
                            },
                        );
                    }
                }
            }
        }
    }
    pub fn retain_participants(&mut self, ids: &[u64]) {
        self.screens
            .retain(|id, v| ids.contains(id) && v.received.elapsed() < Duration::from_secs(4));
        self.remote
            .retain(|id, v| ids.contains(id) && v.received.elapsed() < Duration::from_secs(3));
        self.bridge.retain(&serde_json::to_string(ids).unwrap());
    }
}
impl Drop for Media {
    fn drop(&mut self) {
        self.bridge.stop();
    }
}
