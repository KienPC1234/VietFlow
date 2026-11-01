use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=../../assets/VietFlow.ico");
    println!("cargo:rerun-if-changed=../../assets/VietFlow-TSF.ico");

    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap_or_default();
    let imgui_lib_dir = manifest_dir.join(match target_arch.as_str() {
        "x86" => "../../ui_imgui/build-x86/Release",
        "aarch64" => "../../ui_imgui/build-arm64/Release",
        _ => "../../ui_imgui/build/Release",
    });
    println!(
        "cargo:rerun-if-changed={}",
        imgui_lib_dir.join("vietflow_ui.lib").display()
    );

    let version_path = manifest_dir.join("../../VERSION");
    println!("cargo:rerun-if-changed={}", version_path.display());

    // VERSION is the single source of truth for the app version. Cargo.toml must
    // match so `cargo` metadata, installer version and the exe never diverge.
    let cargo_version = env!("CARGO_PKG_VERSION");
    let version = std::fs::read_to_string(&version_path)
        .map(|text| text.trim().to_string())
        .unwrap_or_else(|_| cargo_version.to_string());

    if version.is_empty() {
        panic!("VERSION file is empty: {}", version_path.display());
    }
    if version != cargo_version {
        panic!(
            "Version mismatch: VERSION says {version}, Cargo.toml says {cargo_version}. \
             Update both before building."
        );
    }
    println!("cargo:rustc-env=VIETFLOW_VERSION={version}");

    if std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default() == "windows" {
        println!("cargo:rustc-link-search=native={}", imgui_lib_dir.display());
        println!("cargo:rustc-link-lib=static=vietflow_ui");
        println!("cargo:rustc-link-lib=dylib=d3d11");
        println!("cargo:rustc-link-lib=dylib=d3dcompiler");
        println!("cargo:rustc-link-lib=dylib=dxgi");
        println!("cargo:rustc-link-lib=dylib=dwmapi");
        println!("cargo:rustc-link-lib=dylib=imm32");
        println!("cargo:rustc-link-lib=dylib=comdlg32");
        println!("cargo:rustc-link-lib=dylib=wininet");
        println!("cargo:rustc-link-lib=dylib=advapi32");

        let numeric: Vec<u32> = version
            .split(|c: char| !c.is_ascii_digit())
            .filter(|part| !part.is_empty())
            .filter_map(|part| part.parse().ok())
            .collect();
        let part = |index: usize| numeric.get(index).copied().unwrap_or(0);
        let win_ver = format!("{}.{}.{}.0", part(0), part(1), part(2));

        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/VietFlow.ico");
        res.set_icon_with_id("../../assets/VietFlow-TSF.ico", "2");
        res.set_language(0x0409);
        res.set("ProductName", "VietFlow");
        res.set("FileDescription", "VietFlow - Modern Vietnamese Input Method");
        res.set("CompanyName", "VietFlow Team");
        res.set("Comments", "VietFlow - Modern Vietnamese Input Method");
        res.set("InternalName", "VietFlow");
        res.set("OriginalFilename", "VietFlow.exe");
        res.set("FileVersion", &win_ver);
        res.set("ProductVersion", &win_ver);
        res.set("LegalCopyright", "Copyright (C) 2026 VietFlow Team. GNU GPL v3.0");

        let manifest = format!(r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0" xmlns:asmv3="urn:schemas-microsoft-com:asm.v3">
  <assemblyIdentity
    version="{win_ver}"
    processorArchitecture="*"
    name="VietFlow.VietFlowTeam.VietFlow"
    type="win32"
  />
  <description>VietFlow - Modern Vietnamese Input Method</description>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity
        type="win32"
        name="Microsoft.Windows.Common-Controls"
        version="6.0.0.0"
        processorArchitecture="*"
        publicKeyToken="6595b64144ccf1df"
        language="*"
      />
    </dependentAssembly>
  </dependency>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <!-- Windows 10 and Windows 11 -->
      <supportedOS Id="{{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}}"/>
      <!-- Windows 8.1 -->
      <supportedOS Id="{{1f676c76-80e1-4239-95bb-83d0f6d0da78}}"/>
      <!-- Windows 8 -->
      <supportedOS Id="{{4a2f28e3-53b9-4441-ba9c-d69d4a4a6e38}}"/>
      <!-- Windows 7 -->
      <supportedOS Id="{{35138b9a-5d96-4fbd-8e2d-a2440225f93a}}"/>
    </application>
  </compatibility>
  <asmv3:application>
    <asmv3:windowsSettings xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">
      <dpiAware xmlns="http://schemas.microsoft.com/SMI/2005/WindowsSettings">true/pm</dpiAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2, PerMonitor</dpiAwareness>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
    </asmv3:windowsSettings>
  </asmv3:application>
</assembly>"#);
        res.set_manifest(&manifest);
        let _ = res.compile();
    }
}
