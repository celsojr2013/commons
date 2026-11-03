use crate::media::Capture;
use serde::{Deserialize, Serialize};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc,
    },
    time::{Duration, Instant},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Device {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Discovery {
    pub authorization: i32,
    pub devices: Vec<Device>,
    pub error: String,
}
impl Discovery {
    pub fn status(&self) -> &'static str {
        match self.authorization {
            0 => "Camera permission has not been requested yet.",
            1 => "Camera access is restricted by macOS.",
            2 => "Camera permission is denied. Open Camera privacy settings to allow Commons.",
            3 => "Camera permission is allowed.",
            _ => "Camera permission status is unavailable.",
        }
    }
}
#[cfg(target_os = "macos")]
mod native {
    use super::*;
    use std::ffi::{c_char, c_void, CStr, CString};
    extern "C" {
        fn commons_camera_list() -> *mut c_char;
        fn commons_camera_free(s: *mut c_char);
        fn commons_camera_permission(cb: extern "C" fn(*mut c_void, i32), ctx: *mut c_void);
        fn commons_camera_start(
            id: *const c_char,
            cb: extern "C" fn(*mut c_void, *const u8, usize),
            ctx: *mut c_void,
            error: *mut *mut c_char,
        ) -> *mut c_void;
        fn commons_camera_stop(handle: *mut c_void);
    }
    fn take_string(pointer: *mut c_char) -> String {
        if pointer.is_null() {
            return String::new();
        }
        unsafe {
            let value = CStr::from_ptr(pointer).to_string_lossy().into_owned();
            commons_camera_free(pointer);
            value
        }
    }
    pub fn discover() -> Result<Discovery, String> {
        let text = take_string(unsafe { commons_camera_list() });
        serde_json::from_str(&text)
            .map_err(|e| format!("Could not read the native camera list: {e}"))
    }
    extern "C" fn permission_result(context: *mut c_void, allowed: i32) {
        let sender = unsafe { Box::from_raw(context as *mut mpsc::SyncSender<bool>) };
        let _ = sender.try_send(allowed == 1);
    }
    pub fn permission(stop: &AtomicBool) -> Result<(), String> {
        let (tx, rx) = mpsc::sync_channel(1);
        let context = Box::into_raw(Box::new(tx)) as *mut c_void;
        unsafe {
            commons_camera_permission(permission_result, context);
        }
        let deadline = Instant::now() + Duration::from_secs(40);
        loop {
            if stop.load(Ordering::Relaxed) {
                return Err("Camera request cancelled.".into());
            }
            match rx.recv_timeout(Duration::from_millis(100)){Ok(true)=>return Ok(()),Ok(false)=>return Err("Allow Commons in System Settings > Privacy & Security > Camera, then refresh devices.".into()),Err(mpsc::RecvTimeoutError::Timeout)if Instant::now()<deadline=>{},_=>return Err("Camera permission is still pending. Check the macOS permission dialog.".into())}
        }
    }
    struct Frames {
        sender: mpsc::SyncSender<Capture>,
        generation: u64,
        started: Instant,
        last: AtomicU64,
        count: AtomicU64,
    }
    extern "C" fn receive_frame(context: *mut c_void, data: *const u8, length: usize) {
        if data.is_null() || length == 0 || length > 98304 {
            return;
        }
        let frames = unsafe { &*(context as *const Frames) };
        let bytes = unsafe { std::slice::from_raw_parts(data, length) }.to_vec();
        frames.last.store(
            frames.started.elapsed().as_millis() as u64 + 1,
            Ordering::Relaxed,
        );
        frames.count.fetch_add(1, Ordering::Relaxed);
        let _ = frames.sender.try_send(Capture::Video {
            generation: frames.generation,
            bytes,
        });
    }
    struct Session {
        handle: *mut c_void,
        frames: *mut Frames,
    }
    impl Drop for Session {
        fn drop(&mut self) {
            unsafe {
                commons_camera_stop(self.handle);
                drop(Box::from_raw(self.frames));
            }
        }
    }
    pub fn run(
        index: String,
        generation: u64,
        events: mpsc::SyncSender<Capture>,
        stop: Arc<AtomicBool>,
    ) -> Result<(), String> {
        permission(&stop)?;
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let name = CString::new(index).map_err(|_| "Camera identifier is invalid.")?;
        let frames = Box::into_raw(Box::new(Frames {
            sender: events.clone(),
            generation,
            started: Instant::now(),
            last: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }));
        let mut error = std::ptr::null_mut();
        let handle = unsafe {
            commons_camera_start(
                name.as_ptr(),
                receive_frame,
                frames as *mut c_void,
                &mut error,
            )
        };
        if handle.is_null() {
            unsafe {
                drop(Box::from_raw(frames));
            }
            let text = take_string(error);
            return Err(if text.is_empty() {
                "The camera could not be opened.".into()
            } else {
                text
            });
        }
        let _session = Session { handle, frames };
        let mut ready = false;
        while !stop.load(Ordering::Relaxed) {
            let state = unsafe { &*frames };
            let elapsed = state.started.elapsed().as_millis() as u64;
            let count = state.count.load(Ordering::Relaxed);
            if count > 0 && !ready {
                ready = true;
                let _ = events.try_send(Capture::CameraReady(generation));
            }
            if !ready && elapsed > 10000 {
                return Err("The camera opened but delivered no frames. Reconnect the USB webcam or check whether another app is using it.".into());
            }
            if ready && elapsed.saturating_sub(state.last.load(Ordering::Relaxed)) > 5000 {
                return Err("The camera stopped delivering frames. Reconnect it and refresh the camera list.".into());
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }
}
#[cfg(target_os = "macos")]
pub use native::{discover, permission, run};
#[cfg(not(target_os = "macos"))]
pub fn discover() -> Result<Discovery, String> {
    let devices = nokhwa::query(nokhwa::utils::ApiBackend::Auto).map_err(|e| e.to_string())?;
    Ok(Discovery {
        authorization: 3,
        devices: devices
            .iter()
            .map(|d| Device {
                id: d.index().as_string(),
                name: d.human_name(),
                kind: String::new(),
            })
            .collect(),
        error: String::new(),
    })
}
#[cfg(not(target_os = "macos"))]
pub fn permission(_stop: &AtomicBool) -> Result<(), String> {
    Ok(())
}
#[cfg(not(target_os = "macos"))]
pub fn run(
    index: String,
    generation: u64,
    events: mpsc::SyncSender<Capture>,
    stop: Arc<AtomicBool>,
) -> Result<(), String> {
    let index = index
        .parse::<u32>()
        .map(nokhwa::utils::CameraIndex::Index)
        .unwrap_or(nokhwa::utils::CameraIndex::String(index));
    let mut camera = nokhwa::Camera::new(
        index,
        nokhwa::utils::RequestedFormat::new::<nokhwa::pixel_format::RgbFormat>(
            nokhwa::utils::RequestedFormatType::None,
        ),
    )
    .map_err(|e| e.to_string())?;
    camera.open_stream().map_err(|e| e.to_string())?;
    let mut last = Instant::now() - Duration::from_secs(1);
    let mut ready = false;
    let result = (|| {
        while !stop.load(Ordering::Relaxed) {
            let frame = camera.frame().map_err(|e| e.to_string())?;
            if last.elapsed() < Duration::from_millis(165) {
                continue;
            }
            let rgb = frame
                .decode_image::<nokhwa::pixel_format::RgbFormat>()
                .map_err(|e| e.to_string())?;
            let image = image::DynamicImage::ImageRgb8(rgb).resize_to_fill(
                320,
                240,
                image::imageops::FilterType::Triangle,
            );
            let mut bytes = vec![];
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 65)
                .encode_image(&image)
                .map_err(|e| e.to_string())?;
            if !ready {
                let _ = events.try_send(Capture::CameraReady(generation));
                ready = true;
            }
            let _ = events.try_send(Capture::Video { generation, bytes });
            last = Instant::now();
        }
        Ok(())
    })();
    let _ = camera.stop_stream();
    result
}

pub fn self_test() -> Result<serde_json::Value, String> {
    let cameras = discover()?;
    if cameras.authorization != 3 {
        return Err("Camera permission must already be allowed before this test; use the in-app access button.".into());
    }
    let selected = cameras.devices.first().ok_or("No camera was found.")?;
    let id = selected.id.clone();
    let name = selected.name.clone();
    let (tx, rx) = mpsc::sync_channel(12);
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let handle = std::thread::spawn(move || run(id, 0, tx, flag));
    let deadline = Instant::now() + Duration::from_secs(12);
    let result = loop {
        match rx.recv_timeout(Duration::from_millis(100)) {
            Ok(Capture::Video { bytes, .. }) => {
                let image = image::load_from_memory(&bytes).map_err(|e| e.to_string());
                break image.map(|image|serde_json::json!({"camera":name,"frame_received":true,"width":image.width(),"height":image.height(),"bytes":bytes.len(),"photo_saved":false}));
            }
            Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() < deadline => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                break Err("Camera stream ended before a frame arrived.".into())
            }
            _ => {
                if Instant::now() >= deadline {
                    break Err("No camera frame arrived before the timeout.".into());
                }
            }
        }
    };
    stop.store(true, Ordering::Relaxed);
    let capture = handle.join().map_err(|_| "Camera worker did not finish.")?;
    capture?;
    result
}
