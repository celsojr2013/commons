use eframe::egui::{ColorImage, TextureHandle};
use serde::{Deserialize, Serialize};
use web_time::Instant;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub microphone: String,
    pub speaker: String,
    pub camera: String,
    pub volume: f32,
    pub mirror: bool,
    pub quality: CameraQuality,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            microphone: String::new(),
            speaker: String::new(),
            camera: "0".into(),
            volume: 0.7,
            mirror: true,
            quality: CameraQuality::Low,
        }
    }
}
#[derive(Default)]
pub struct Devices {
    pub microphones: Vec<String>,
    pub speakers: Vec<String>,
    pub cameras: Vec<(String, String)>,
}
pub struct VideoView {
    pub texture: TextureHandle,
    pub received: Instant,
}
pub fn decode_video(bytes: &[u8], mirror: bool) -> Result<ColorImage, String> {
    if bytes.len() > 524288 {
        return Err("Video frame is too large.".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(1920);
    limits.max_image_height = Some(1920);
    limits.max_alloc = Some(24 * 1024 * 1024);
    reader.limits(limits);
    let mut image = reader.decode().map_err(|e| e.to_string())?.to_rgba8();
    if mirror {
        image::imageops::flip_horizontal_in_place(&mut image);
    }
    Ok(ColorImage::from_rgba_unmultiplied(
        [image.width() as usize, image.height() as usize],
        image.as_raw(),
    ))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraQuality {
    #[default]
    Low,
    Medium,
    High,
}
impl CameraQuality {
    pub fn profile(self) -> (u32, u32, u32, u8) {
        match self {
            Self::Low => (320, 240, 6, 60),
            Self::Medium => (640, 480, 15, 65),
            Self::High => (1280, 720, 24, 70),
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "Low · 320×240 / 6 fps",
            Self::Medium => "Medium · 640×480 / 15 fps",
            Self::High => "High · 1280×720 / 24 fps",
        }
    }
}
#[derive(Clone, Debug)]
pub struct ScreenSource {
    pub id: String,
    pub name: String,
}
