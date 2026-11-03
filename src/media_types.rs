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
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            microphone: String::new(),
            speaker: String::new(),
            camera: "0".into(),
            volume: 0.7,
            mirror: true,
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
    if bytes.len() > 98304 {
        return Err("Video frame is too large.".into());
    }
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(640);
    limits.max_image_height = Some(480);
    limits.max_alloc = Some(8 * 1024 * 1024);
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
