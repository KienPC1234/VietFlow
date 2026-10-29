fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/VietFlow.ico");
        res.set("ProductName", "VietFlow");
        res.set("FileDescription", "VietFlow - Bộ gõ tiếng Việt thế hệ mới");
        res.set("LegalCopyright", "Copyright (C) 2026 VietFlow Team. GNU GPL v3.0");
        let _ = res.compile();
    }
}

