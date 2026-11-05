use crate::*;
use wasm_bindgen::prelude::*;
#[wasm_bindgen]
pub struct WebHandle {
    runner: eframe::WebRunner,
}
#[wasm_bindgen]
impl WebHandle {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        console_error_panic_hook::set_once();
        Self {
            runner: eframe::WebRunner::new(),
        }
    }
    pub async fn start(
        &self,
        canvas: web_sys::HtmlCanvasElement,
        name: String,
        password: String,
    ) -> Result<(), JsValue> {
        prepare_canvas(&canvas)?;
        self.runner
            .start(
                canvas,
                eframe::WebOptions {
                    dithering: false,
                    ..Default::default()
                },
                Box::new(move |cc| {
                    let mut app = Commons::new(&cc.egui_ctx, None);
                    app.settings.name = if name.trim().is_empty() {
                        "Guest".into()
                    } else {
                        name.chars().take(32).collect()
                    };
                    app.settings.address = origin();
                    app.password = password;
                    app.connect(origin());
                    Ok(Box::new(app))
                }),
            )
            .await
    }
    pub fn destroy(&self) {
        self.runner.destroy();
    }
}
pub fn origin() -> String {
    web_sys::window()
        .and_then(|w| w.location().origin().ok())
        .unwrap_or_default()
}
#[wasm_bindgen(module = "/web/download.js")]
extern "C" {
    #[wasm_bindgen(catch)]
    fn prepare_canvas(canvas: &web_sys::HtmlCanvasElement) -> Result<(), JsValue>;
    fn download(name: &str, mime: &str, bytes: &[u8]);
    fn report_status(json: &str);
}
pub fn publish(app: &Commons) {
    let mut status = serde_json::json!({"connected":app.connected,"connecting":app.connecting.is_some(),"client_only":true,"people":app.people().len(),"room":app.room,"position":app.position,"name":app.settings.name,"error":app.error,"mic":app.media.mic_on,"camera":app.media.camera_on,"preview":app.media.preview,"camera_ready":app.media.camera_ready,"mic_ready":app.media.mic_ready,"remote_video":app.media.remote.len(),"remote_screens":app.media.screens.len(),"screen":app.media.screen_on,"screen_ready":app.media.screen_ready,"admin":app.is_admin,"maps":app.maps.len(),"meeting":app.meeting.active,"meeting_page":app.meeting.page,"pin":format!("{:?}",app.meeting.pin),"quality":format!("{:?}",app.media.preferences.quality)});
    if app.qa {
        status["local_camera_size"] =
            serde_json::json!(app.media.local_video.as_ref().map(|v| v.texture.size()));
        status["local_screen_size"] =
            serde_json::json!(app.media.local_screen.as_ref().map(|v| v.texture.size()));
        status["frame"] = serde_json::json!(app.frames);
        status["controls"] = serde_json::json!(app
            .chrome
            .targets
            .iter()
            .map(|(k, r)| (*k, [r.left(), r.top(), r.width(), r.height()]))
            .collect::<std::collections::HashMap<_, _>>());
        status["map_world"] = serde_json::json!(app.chrome.world_rect.map(|r| [
            r.left(),
            r.top(),
            r.width(),
            r.height()
        ]));
        status["profile_open"] = serde_json::json!(app.chrome.profile_open);
        status["call_open"] = serde_json::json!(app.show_call);
        status["call_collapsed"] = serde_json::json!(app.chrome.call_collapsed);
        status["scope"] = serde_json::json!(app.current_scope());
    }
    report_status(&status.to_string());
}
pub fn export(app: &Commons) -> Result<(), String> {
    let mut data = std::io::Cursor::new(vec![]);
    image::DynamicImage::ImageRgba8(app.sprites.compose(&app.avatar_draft))
        .write_to(&mut data, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    download("my-avatar.png", "image/png", data.get_ref());
    let credits = format!(
        "{}\n\n{}",
        include_str!("../assets/ASSET_CREDITS.txt"),
        include_str!("../assets/LICENSES/CC-BY-SA-4.0.txt")
    );
    download(
        "avatar-art-credits.txt",
        "text/plain;charset=utf-8",
        credits.as_bytes(),
    );
    Ok(())
}
