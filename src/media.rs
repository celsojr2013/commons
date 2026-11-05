pub use crate::media_types::*;
use crate::{
    network::{MediaKind, MediaPacket},
    world::TalkScope,
};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use eframe::egui::{Context, TextureOptions};
use std::{
    collections::{HashMap, VecDeque},
    sync::{
        atomic::{AtomicBool, AtomicU32, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};
pub enum Capture {
    Audio {
        generation: u64,
        bytes: Vec<u8>,
    },
    Video {
        generation: u64,
        bytes: Vec<u8>,
    },
    Screen {
        generation: u64,
        bytes: Vec<u8>,
    },
    MicReady(u64),
    CameraReady(u64),
    CameraPermission(Result<(), String>),
    Error {
        source: &'static str,
        generation: u64,
        text: String,
    },
}
enum AudioCommand {
    Input {
        device: Option<String>,
        generation: u64,
    },
    Output(Option<String>),
    Tone,
    Stop,
}
#[derive(Default)]
struct Voice {
    samples: VecDeque<f32>,
    phase: f64,
}
#[derive(Default)]
struct Mixer {
    voices: HashMap<u64, Voice>,
    tone_left: usize,
    tone_phase: f32,
}
impl Mixer {
    fn push(&mut self, id: u64, bytes: &[u8]) {
        if bytes.len() != 640 {
            return;
        }
        let voice = self.voices.entry(id).or_default();
        voice.samples.extend(
            bytes
                .chunks_exact(2)
                .map(|b| i16::from_le_bytes([b[0], b[1]]) as f32 / 32768.0),
        );
        if voice.samples.len() > 3200 {
            let n = voice.samples.len() - 1600;
            voice.samples.drain(..n);
        }
    }
    fn sample(&mut self, rate: u32) -> f32 {
        let mut sum = 0.0;
        for voice in self.voices.values_mut() {
            if let Some(s) = voice.samples.front() {
                sum += *s;
                voice.phase += 16000.0 / rate as f64;
                while voice.phase >= 1.0 {
                    voice.samples.pop_front();
                    voice.phase -= 1.0;
                }
            }
        }
        if self.tone_left > 0 {
            sum += (self.tone_phase * std::f32::consts::TAU).sin() * 0.15;
            self.tone_phase = (self.tone_phase + 440.0 / rate as f32) % 1.0;
            self.tone_left -= 1;
        }
        sum.clamp(-1.0, 1.0)
    }
}
struct AudioEngine {
    tx: mpsc::Sender<AudioCommand>,
    handle: Option<thread::JoinHandle<()>>,
}
impl AudioEngine {
    fn new(
        events: mpsc::SyncSender<Capture>,
        mixer: Arc<Mutex<Mixer>>,
        level: Arc<AtomicU32>,
        volume: Arc<AtomicU32>,
    ) -> Self {
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut input = None;
            let mut output = None;
            while let Ok(cmd) = rx.recv() {
                match cmd {
                    AudioCommand::Stop => break,
                    AudioCommand::Input { device, generation } => {
                        input.take();
                        level.store(0.0f32.to_bits(), Ordering::Relaxed);
                        if let Some(name) = device {
                            match make_input(&name, events.clone(), level.clone(), generation) {
                                Ok(stream) => {
                                    input = Some(stream);
                                    let _ = events.try_send(Capture::MicReady(generation));
                                }
                                Err(text) => {
                                    let _ = events.try_send(Capture::Error {
                                        source: "microphone",
                                        generation,
                                        text,
                                    });
                                }
                            }
                        }
                    }
                    AudioCommand::Output(device) => {
                        output.take();
                        if let Some(name) = device {
                            match make_output(&name, mixer.clone(), volume.clone()) {
                                Ok(stream) => output = Some(stream),
                                Err(text) => {
                                    let _ = events.try_send(Capture::Error {
                                        source: "speakers",
                                        generation: 0,
                                        text,
                                    });
                                }
                            }
                        }
                    }
                    AudioCommand::Tone => {
                        if let Ok(mut m) = mixer.lock() {
                            m.tone_left = 48000 / 3;
                        }
                    }
                }
            }
        });
        Self {
            tx,
            handle: Some(handle),
        }
    }
}
impl Drop for AudioEngine {
    fn drop(&mut self) {
        let _ = self.tx.send(AudioCommand::Stop);
        if let Some(h) = self.handle.take() {
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
}
fn select_audio(name: &str, input: bool) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    if name.is_empty() {
        return (if input {
            host.default_input_device()
        } else {
            host.default_output_device()
        })
        .ok_or_else(|| {
            if input {
                "No microphone is available.".into()
            } else {
                "No speaker output is available.".into()
            }
        });
    }
    let devices = if input {
        host.input_devices()
    } else {
        host.output_devices()
    }
    .map_err(|e| e.to_string())?;
    devices
        .into_iter()
        .find(|d| d.name().is_ok_and(|n| n == name))
        .ok_or_else(|| format!("{name} is no longer available. Refresh the device list."))
}
fn make_input(
    name: &str,
    events: mpsc::SyncSender<Capture>,
    level: Arc<AtomicU32>,
    generation: u64,
) -> Result<cpal::Stream, String> {
    let device = select_audio(name, true)?;
    let format = device
        .default_input_config()
        .map_err(|e| format!("Could not open this microphone: {e}"))?;
    let config: cpal::StreamConfig = format.clone().into();
    let stream = match format.sample_format() {
        cpal::SampleFormat::F32 => input_stream::<f32>(&device, &config, events, level, generation),
        cpal::SampleFormat::I16 => input_stream::<i16>(&device, &config, events, level, generation),
        cpal::SampleFormat::U16 => input_stream::<u16>(&device, &config, events, level, generation),
        cpal::SampleFormat::I32 => input_stream::<i32>(&device, &config, events, level, generation),
        _ => {
            return Err(
                "This microphone's sample format is not supported. Choose another microphone."
                    .into(),
            )
        }
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
fn input_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    events: mpsc::SyncSender<Capture>,
    level: Arc<AtomicU32>,
    generation: u64,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample,
    f32: cpal::FromSample<T>,
{
    let channels = config.channels as usize;
    let rate = config.sample_rate.0 as f64;
    let mut phase = 0.0;
    let mut sum = 0.0f32;
    let mut count = 0;
    let mut bytes = Vec::with_capacity(640);
    let mut energy = 0.0;
    let errors = events.clone();
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                for frame in data.chunks(channels) {
                    let sample =
                        frame.iter().map(|s| s.to_sample::<f32>()).sum::<f32>() / channels as f32;
                    sum += sample;
                    count += 1;
                    phase += 16000.0;
                    if phase >= rate {
                        phase -= rate;
                        let sample = (sum / count as f32).clamp(-1.0, 1.0);
                        sum = 0.0;
                        count = 0;
                        energy += sample * sample;
                        bytes.extend_from_slice(&((sample * 32767.0) as i16).to_le_bytes());
                        if bytes.len() == 640 {
                            level.store((energy / 320.0f32).sqrt().to_bits(), Ordering::Relaxed);
                            energy = 0.0;
                            let packet = std::mem::replace(&mut bytes, Vec::with_capacity(640));
                            let _ = events.try_send(Capture::Audio {
                                generation,
                                bytes: packet,
                            });
                        }
                    }
                }
            },
            move |e| {
                let _ = errors.try_send(Capture::Error {
                    source: "microphone",
                    generation,
                    text: format!("The microphone stopped: {e}"),
                });
            },
            None,
        )
        .map_err(|e| {
            format!(
                "Microphone access failed: {e}. Check microphone permission in System Settings."
            )
        })
}
fn make_output(
    name: &str,
    mixer: Arc<Mutex<Mixer>>,
    volume: Arc<AtomicU32>,
) -> Result<cpal::Stream, String> {
    let device = select_audio(name, false)?;
    let format = device.default_output_config().map_err(|e| e.to_string())?;
    let config: cpal::StreamConfig = format.clone().into();
    let stream = match format.sample_format() {
        cpal::SampleFormat::F32 => output_stream::<f32>(&device, &config, mixer, volume),
        cpal::SampleFormat::I16 => output_stream::<i16>(&device, &config, mixer, volume),
        cpal::SampleFormat::U16 => output_stream::<u16>(&device, &config, mixer, volume),
        cpal::SampleFormat::I32 => output_stream::<i32>(&device, &config, mixer, volume),
        _ => return Err("Choose a speaker output with a standard audio format.".into()),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok(stream)
}
fn output_stream<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    mixer: Arc<Mutex<Mixer>>,
    volume: Arc<AtomicU32>,
) -> Result<cpal::Stream, String>
where
    T: cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = config.channels as usize;
    let rate = config.sample_rate.0;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let gain = f32::from_bits(volume.load(Ordering::Relaxed));
                let mut mixer = mixer.try_lock().ok();
                for frame in data.chunks_mut(channels) {
                    let sample = mixer.as_mut().map_or(0.0, |m| m.sample(rate)) * gain;
                    for output in frame {
                        *output = T::from_sample(sample);
                    }
                }
            },
            |e| eprintln!("Audio output: {e}"),
            None,
        )
        .map_err(|e| format!("Could not play audio: {e}"))
}
struct CameraRun {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}
impl CameraRun {
    fn start(
        index: String,
        generation: u64,
        quality: CameraQuality,
        events: mpsc::SyncSender<Capture>,
    ) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            let result =
                crate::camera_backend::run(index, generation, quality, events.clone(), flag);
            if let Err(text) = result {
                let _ = events.try_send(Capture::Error {
                    source: "camera",
                    generation,
                    text,
                });
            }
        });
        Self {
            stop,
            handle: Some(handle),
        }
    }
}
impl Drop for CameraRun {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            if h.is_finished() {
                let _ = h.join();
            }
        }
    }
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
    screen: Option<CameraRun>,
    screen_generation: u64,
    pub mic_test: bool,
    pub preview: bool,
    pub listening: bool,
    pub mic_ready: bool,
    pub camera_ready: bool,
    pub local_video: Option<VideoView>,
    pub remote: HashMap<u64, VideoView>,
    scope: Option<TalkScope>,
    audio: AudioEngine,
    camera: Option<CameraRun>,
    events: mpsc::Receiver<Capture>,
    sender: mpsc::SyncSender<Capture>,
    mixer: Arc<Mutex<Mixer>>,
    level: Arc<AtomicU32>,
    volume: Arc<AtomicU32>,
    input_generation: u64,
    camera_generation: u64,
}
impl Media {
    pub fn new(preferences: Preferences) -> Self {
        let (tx, events) = mpsc::sync_channel(12);
        let mixer = Arc::new(Mutex::new(Mixer::default()));
        let level = Arc::new(AtomicU32::new(0));
        let volume = Arc::new(AtomicU32::new(preferences.volume.clamp(0.0, 1.0).to_bits()));
        let audio = AudioEngine::new(tx.clone(), mixer.clone(), level.clone(), volume.clone());
        Self {
            preferences,
            devices: Devices::default(),
            error: String::new(),
            camera_status: "Camera access has not been checked yet.".into(),
            mic_on: false,
            camera_on: false,
            screen_on: false,
            screen_ready: false,
            screen_sources: vec![],
            screen_source: String::new(),
            local_screen: None,
            screens: HashMap::new(),
            screen: None,
            screen_generation: 0,
            mic_test: false,
            preview: false,
            listening: true,
            mic_ready: false,
            camera_ready: false,
            local_video: None,
            remote: HashMap::new(),
            scope: None,
            audio,
            camera: None,
            events,
            sender: tx,
            mixer,
            level,
            volume,
            input_generation: 0,
            camera_generation: 0,
        }
    }
    pub fn refresh_devices(&mut self) {
        let host = cpal::default_host();
        self.devices.microphones = host
            .input_devices()
            .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        self.devices.speakers = host
            .output_devices()
            .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
            .unwrap_or_default();
        self.refresh_cameras();
    }
    pub fn refresh_cameras(&mut self) {
        match crate::camera_backend::discover() {
            Ok(result) => {
                self.camera_status = format!(
                    "{} {} camera(s) found.",
                    result.status(),
                    result.devices.len()
                );
                self.devices.cameras = result.devices.into_iter().map(|d| (d.id, d.name)).collect();
                if !result.error.is_empty() {
                    self.error = result.error;
                }
            }
            Err(error) => {
                self.devices.cameras.clear();
                self.camera_status = error.clone();
                self.error = error;
            }
        }
    }
    pub fn request_camera_access(&mut self) {
        let sender = self.sender.clone();
        std::thread::spawn(move || {
            let result = crate::camera_backend::permission(&AtomicBool::new(false));
            let _ = sender.send(Capture::CameraPermission(result));
        });
    }
    pub fn update_input(&mut self) {
        self.input_generation += 1;
        self.mic_ready = false;
        let device = (self.mic_on || self.mic_test).then(|| self.preferences.microphone.clone());
        let _ = self.audio.tx.send(AudioCommand::Input {
            device,
            generation: self.input_generation,
        });
    }
    pub fn update_camera(&mut self) {
        self.camera_generation += 1;
        self.camera = None;
        self.camera_ready = false;
        self.local_video = None;
        if self.camera_on || self.preview {
            self.camera = Some(CameraRun::start(
                self.preferences.camera.clone(),
                self.camera_generation,
                self.preferences.quality,
                self.sender.clone(),
            ));
        }
    }
    pub fn refresh_screens(&mut self) {
        match crate::screen_backend::sources() {
            Ok(sources) => self.screen_sources = sources,
            Err(error) => self.error = error,
        }
    }
    pub fn start_screen(&mut self) {
        self.stop_screen();
        if self.screen_source.is_empty() {
            self.error = "Select a screen or window.".into();
            return;
        }
        self.screen_on = true;
        let source = self.screen_source.clone();
        let generation = self.screen_generation;
        let events = self.sender.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let handle = thread::spawn(move || {
            if let Err(text) = crate::screen_backend::run(source, generation, events.clone(), flag)
            {
                let _ = events.send(Capture::Error {
                    source: "screen",
                    generation,
                    text,
                });
            }
        });
        self.screen = Some(CameraRun {
            stop,
            handle: Some(handle),
        });
    }
    pub fn stop_screen(&mut self) {
        self.screen_generation = self.screen_generation.wrapping_add(1);
        self.screen_on = false;
        self.screen_ready = false;
        self.screen = None;
        self.local_screen = None;
    }
    pub fn update_output(&mut self) {
        self.volume.store(
            self.preferences.volume.clamp(0.0, 1.0).to_bits(),
            Ordering::Relaxed,
        );
        let device = self.listening.then(|| self.preferences.speaker.clone());
        let _ = self.audio.tx.send(AudioCommand::Output(device));
    }
    pub fn test_speaker(&mut self) {
        self.listening = true;
        self.update_output();
        let _ = self.audio.tx.send(AudioCommand::Tone);
    }
    pub fn level(&self) -> f32 {
        f32::from_bits(self.level.load(Ordering::Relaxed)).clamp(0.0, 1.0)
    }
    pub fn set_volume(&self) {
        self.volume.store(
            self.preferences.volume.clamp(0.0, 1.0).to_bits(),
            Ordering::Relaxed,
        );
    }
    pub fn scope(&self) -> Option<&TalkScope> {
        self.scope.as_ref()
    }
    pub fn pause_sharing(&mut self) -> bool {
        let paused = self.mic_on || self.camera_on || self.screen_on;
        self.stop_screen();
        self.mic_on = false;
        self.camera_on = false;
        self.update_input();
        if self.camera.is_some() {
            self.update_camera();
        }
        self.clear_remote();
        paused
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
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.voices.clear();
        }
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
        self.camera = None;
        self.camera_generation += 1;
        self.local_video = None;
        self.update_input();
        self.clear_remote();
        let _ = self.audio.tx.send(AudioCommand::Output(None));
        self.scope = None;
    }
    pub fn poll(&mut self, ctx: &Context, connected: bool) -> Vec<MediaPacket> {
        let mut outgoing = vec![];
        let mut stop_input = false;
        let mut stop_camera = false;
        let mut stop_screen = false;
        let mut refresh_cameras = false;
        for event in self.events.try_iter() {
            match event {
                Capture::CameraPermission(result) => {
                    match result {
                        Ok(()) => {
                            self.error.clear();
                        }
                        Err(error) => self.error = error,
                    }
                    refresh_cameras = true;
                }
                Capture::Audio { generation, bytes }
                    if generation == self.input_generation && self.mic_on && connected =>
                {
                    outgoing.push(MediaPacket::from_bytes(MediaKind::Audio, &bytes));
                }
                Capture::Video { generation, bytes } if generation == self.camera_generation => {
                    if let Ok(image) = decode_video(&bytes, self.preferences.mirror) {
                        let texture =
                            ctx.load_texture("local camera", image, TextureOptions::LINEAR);
                        self.local_video = Some(VideoView {
                            texture,
                            received: Instant::now(),
                        });
                    }
                    if self.camera_on && connected {
                        outgoing.push(MediaPacket::from_bytes(MediaKind::Video, &bytes));
                    }
                }
                Capture::Screen { generation, bytes }
                    if generation == self.screen_generation && self.screen_on =>
                {
                    self.screen_ready = true;
                    if let Ok(image) = decode_video(&bytes, false) {
                        let texture =
                            ctx.load_texture("local screen", image, TextureOptions::LINEAR);
                        self.local_screen = Some(VideoView {
                            texture,
                            received: Instant::now(),
                        });
                    }
                    if connected {
                        outgoing.push(MediaPacket::from_bytes(MediaKind::Screen, &bytes));
                    }
                }
                Capture::MicReady(g) if g == self.input_generation => self.mic_ready = true,
                Capture::CameraReady(g) if g == self.camera_generation => self.camera_ready = true,
                Capture::Error {
                    source,
                    generation,
                    text,
                } => {
                    let current = match source {
                        "microphone" => generation == self.input_generation,
                        "camera" => generation == self.camera_generation,
                        "screen" => generation == self.screen_generation,
                        _ => true,
                    };
                    if current {
                        self.error = text;
                        match source {
                            "microphone" => {
                                self.mic_on = false;
                                self.mic_test = false;
                                self.mic_ready = false;
                                stop_input = true;
                            }
                            "camera" => {
                                self.camera_on = false;
                                self.preview = false;
                                self.camera_ready = false;
                                stop_camera = true;
                            }
                            "screen" => stop_screen = true,
                            _ => self.listening = false,
                        }
                    }
                }
                _ => {}
            }
        }
        if stop_screen {
            self.stop_screen();
        }
        if refresh_cameras {
            self.refresh_cameras();
        }
        if stop_input {
            self.update_input();
        }
        if stop_camera {
            self.update_camera();
        }
        outgoing
    }
    pub fn receive(&mut self, ctx: &Context, sender: u64, scope: &TalkScope, packet: &MediaPacket) {
        if self.scope.as_ref() != Some(scope) {
            return;
        }
        let Ok(bytes) = packet.bytes() else {
            return;
        };
        match packet.kind {
            MediaKind::Audio => {
                if self.listening {
                    if let Ok(mut mixer) = self.mixer.lock() {
                        mixer.push(sender, &bytes);
                    }
                }
            }
            MediaKind::Video | MediaKind::Screen => {
                if let Ok(image) = decode_video(&bytes, false) {
                    let texture =
                        ctx.load_texture(format!("camera {sender}"), image, TextureOptions::LINEAR);
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
    pub fn retain_participants(&mut self, ids: &[u64]) {
        self.screens.retain(|id, view| {
            ids.contains(id) && view.received.elapsed() < Duration::from_secs(4)
        });
        self.remote.retain(|id, view| {
            ids.contains(id) && view.received.elapsed() < Duration::from_secs(3)
        });
        if let Ok(mut mixer) = self.mixer.lock() {
            mixer.voices.retain(|id, _| ids.contains(id));
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn mixer_buffers_are_bounded_and_do_not_repeat_old_audio() {
        let mut m = Mixer::default();
        let bytes: Vec<_> = (0..320).flat_map(|_| 16000i16.to_le_bytes()).collect();
        for _ in 0..20 {
            m.push(1, &bytes);
        }
        assert!(m.voices[&1].samples.len() <= 3200);
        assert!(m.sample(48000) > 0.4);
        for _ in 0..12000 {
            m.sample(48000);
        }
        assert_eq!(m.sample(48000), 0.0);
        m.voices.clear();
        assert_eq!(m.sample(48000), 0.0);
    }
    #[test]
    fn video_codec_and_packet_roundtrip() {
        let image = image::RgbImage::from_pixel(320, 240, image::Rgb([12, 42, 90]));
        let mut bytes = vec![];
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 65)
            .encode_image(&image)
            .unwrap();
        let packet = MediaPacket::from_bytes(MediaKind::Video, &bytes);
        assert_eq!(packet.bytes().unwrap(), bytes);
        let image = decode_video(&bytes, false).unwrap();
        assert_eq!(image.size, [320, 240]);
        assert!(decode_video(b"not a camera frame", false).is_err());
    }
    #[test]
    fn conversation_changes_pause_capture_and_discard_old_generation() {
        let ctx = Context::default();
        let mut media = Media::new(Preferences::default());
        assert!(!media.mic_on && !media.camera_on);
        media.change_scope(Some(TalkScope {
            room: 0,
            area: Some(9),
        }));
        let old = media.input_generation;
        media.mic_on = true;
        media.camera_on = true;
        assert!(media.change_scope(Some(TalkScope {
            room: 0,
            area: None
        })));
        assert!(!media.mic_on && !media.camera_on);
        media.mic_on = true;
        media
            .sender
            .try_send(Capture::Audio {
                generation: old,
                bytes: vec![0; 640],
            })
            .unwrap();
        assert!(media.poll(&ctx, true).is_empty());
        media.mic_on = false;
        media.mic_test = true;
        media.preview = true;
        media.end_tests();
        assert!(!media.mic_test && !media.preview);
        assert!(media.camera.is_none());
    }
}
