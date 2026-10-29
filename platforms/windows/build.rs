use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is set by Cargo"),
    );

    // Optional single-file payload: scripts/build.ps1 creates this archive from
    // the deployed Qt UI. When absent, the host looks for vietflow-ui.exe next
    // to itself (development layout).
    let payload = manifest_dir.join("../../ui-runtime.zip");
    println!("cargo:rerun-if-changed={}", payload.display());
    println!("cargo:rerun-if-changed=../../assets/VietFlow.ico");

    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR is set by Cargo"));
    let payload_rs = if payload.is_file() {
        format!(
            "pub static UI_PAYLOAD: &[u8] = include_bytes!({:?});\n",
            payload.to_string_lossy()
        )
    } else {
        "pub static UI_PAYLOAD: &[u8] = &[];\n".to_string()
    };
    std::fs::write(out_dir.join("ui_payload.rs"), payload_rs)
        .expect("failed to write ui_payload.rs");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/VietFlow.ico");
        res.set("ProductName", "VietFlow");
        res.set("FileDescription", "VietFlow - Bộ gõ tiếng Việt");
        res.set("FileVersion", "0.1.0.0");
        res.set("ProductVersion", "0.1.0.0");
        res.set("LegalCopyright", "Copyright (C) 2026 VietFlow Team. GNU GPL v3.0");
        let _ = res.compile();
    }
}
