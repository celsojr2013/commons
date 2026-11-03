fn main() {
    use std::{fs, path::Path};
    fn walk(dir: &Path, root: &Path, rows: &mut Vec<String>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, root, rows);
            } else if !p.to_string_lossy().ends_with(".gz") {
                let name = format!("/{}", p.strip_prefix(root).unwrap().to_string_lossy());
                let gz = p.with_extension(format!(
                    "{}.gz",
                    p.extension().unwrap_or_default().to_string_lossy()
                ));
                let compressed = if gz.exists() {
                    format!("include_bytes!({:?})", gz)
                } else {
                    "&[]".into()
                };
                rows.push(format!("({name:?},include_bytes!({p:?}),{compressed}),"));
            }
        }
    }
    let root =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap()).join("web/dist");
    println!("cargo:rerun-if-changed=web/dist");
    let mut rows = vec![];
    walk(&root, &root, &mut rows);
    rows.sort();
    fs::write(
        Path::new(&std::env::var("OUT_DIR").unwrap()).join("web_assets.rs"),
        format!(
            "static ASSETS: &[(&str,&[u8],&[u8])] = &[{}];",
            rows.join("\n")
        ),
    )
    .unwrap();

    println!("cargo:rerun-if-changed=platform/Info.plist");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=platform/camera.m");
        cc::Build::new()
            .file("platform/camera.m")
            .flag("-fobjc-arc")
            .flag("-fblocks")
            .compile("commons_camera");
        for framework in [
            "AVFoundation",
            "AppKit",
            "CoreImage",
            "CoreMedia",
            "CoreVideo",
        ] {
            println!("cargo:rustc-link-lib=framework={framework}");
        }
        let path = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("platform/Info.plist");
        println!(
            "cargo:rustc-link-arg=-Wl,-sectcreate,__TEXT,__info_plist,{}",
            path.display()
        );
    }
}
