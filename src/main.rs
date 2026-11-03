#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    commons_desktop::run_native()
}
#[cfg(target_arch = "wasm32")]
fn main() {}
