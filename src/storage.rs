use serde::Serialize;
use std::path::{Path, PathBuf};
#[cfg(not(target_arch = "wasm32"))]
pub fn data_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("COMMONS_DATA_DIR") {
        return PathBuf::from(p);
    }
    let project = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data");
    if project.exists() {
        return project;
    }
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into()));
    if cfg!(target_os = "macos") {
        home.join("Library/Application Support/Commons")
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".local/share"))
            .join("commons")
    }
}
#[cfg(target_arch = "wasm32")]
pub fn data_dir() -> PathBuf {
    PathBuf::from("commons-browser")
}
#[cfg(not(target_arch = "wasm32"))]
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|e| e.to_string())
}
#[cfg(target_arch = "wasm32")]
pub fn read(path: &Path) -> Result<Vec<u8>, String> {
    let key = format!(
        "commons-v4/{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage is unavailable")?
        .ok_or("Browser storage is disabled")?
        .get_item(&key)
        .map_err(|_| "Could not read browser preferences")?
        .map(|s| s.into_bytes())
        .ok_or_else(|| "No saved preference".into())
}
#[cfg(not(target_arch = "wasm32"))]
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    std::fs::create_dir_all(path.parent().ok_or("No parent")?).map_err(|e| e.to_string())?;
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, bytes).map_err(|e| e.to_string())?;
    std::fs::rename(temp, path).map_err(|e| e.to_string())
}
#[cfg(target_arch = "wasm32")]
pub fn write<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let key = format!(
        "commons-v4/{}",
        path.file_name().unwrap_or_default().to_string_lossy()
    );
    let text = serde_json::to_string(value).map_err(|e| e.to_string())?;
    web_sys::window()
        .ok_or("Browser window unavailable")?
        .local_storage()
        .map_err(|_| "Browser storage is unavailable")?
        .ok_or("Browser storage is disabled")?
        .set_item(&key, &text)
        .map_err(|_| "Browser storage is full or disabled".into())
}
