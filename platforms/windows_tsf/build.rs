fn main() {
    println!("cargo:rerun-if-changed=../../assets/VietFlow-TSF.ico");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/VietFlow-TSF.ico");
        res.set("ProductName", "VietFlow TSF");
        res.set("FileDescription", "VietFlow Text Services Framework TIP");
        let _ = res.compile();

        if std::env::var("CARGO_CFG_TARGET_ENV").unwrap_or_default() == "msvc" {
            println!("cargo:rustc-cdylib-link-arg=/IGNORE:4104");
        }
    }
}

