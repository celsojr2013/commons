use crate::media::{Capture, ScreenSource};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use std::time::{Duration, Instant};

pub fn sources() -> Result<Vec<ScreenSource>, String> {
    let mut found = vec![];
    for monitor in xcap::Monitor::all().map_err(|e| e.to_string())? {
        found.push(ScreenSource {
            id: format!("monitor:{}", monitor.id().map_err(|e| e.to_string())?),
            name: format!(
                "Screen · {}",
                monitor.name().unwrap_or_else(|_| "Display".into())
            ),
        });
    }
    for window in xcap::Window::all().map_err(|e| e.to_string())? {
        if window.is_minimized().unwrap_or(true) {
            continue;
        }
        let title = window.title().unwrap_or_default();
        if title.trim().is_empty() {
            continue;
        }
        found.push(ScreenSource {
            id: format!("window:{}", window.id().map_err(|e| e.to_string())?),
            name: format!("{} · {}", window.app_name().unwrap_or_default(), title),
        });
    }
    Ok(found)
}

pub fn run(
    source: String,
    generation: u64,
    events: mpsc::SyncSender<Capture>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let (kind, id) = source.split_once(':').ok_or("Select a screen or window.")?;
    let id: u32 = id.parse().map_err(|_| "Invalid screen source.")?;
    enum Source {
        Monitor(xcap::Monitor),
        Window(xcap::Window),
    }
    let selected = match kind {
        "monitor" => Source::Monitor(
            xcap::Monitor::all()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|s| s.id().ok() == Some(id))
                .ok_or("The selected display is no longer available.")?,
        ),
        "window" => Source::Window(
            xcap::Window::all()
                .map_err(|e| e.to_string())?
                .into_iter()
                .find(|s| s.id().ok() == Some(id))
                .ok_or("The selected window is no longer available.")?,
        ),
        _ => return Err("Select a screen or window.".into()),
    };
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        let frame = match &selected {
            Source::Monitor(m) => m.capture_image(),
            Source::Window(w) => w.capture_image(),
        }.map_err(|e| format!("Screen capture stopped: {e}. Check screen-recording permission and the selected window."))?;
        if stop.load(Ordering::Relaxed) {
            break;
        }
        let image = image::DynamicImage::ImageRgba8(frame)
            .resize(1920, 1080, image::imageops::FilterType::Triangle)
            .to_rgb8();
        let mut bytes = vec![];
        for quality in [80, 65, 45] {
            bytes.clear();
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
                .encode_image(&image)
                .map_err(|e| e.to_string())?;
            if bytes.len() <= 524288 {
                break;
            }
        }
        if bytes.len() <= 524288 && !stop.load(Ordering::Relaxed) {
            let _ = events.try_send(Capture::Screen { generation, bytes });
        }
        // At most five frames/sec; readable document sharing, independent of webcam quality.
        while started.elapsed() < Duration::from_millis(200) && !stop.load(Ordering::Relaxed) {
            std::thread::sleep(Duration::from_millis(10));
        }
    }
    Ok(())
}
