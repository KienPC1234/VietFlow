# VietFlow

Bộ gõ tiếng Việt hiệu năng cao với kiến trúc Rust Core kết hợp Qt 6 Configuration UI và Windows Low-Level Keyboard Hook.

Mã nguồn: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

---

## Kiến trúc & Cấu trúc mã nguồn

```
vietflow/
├── core/                  Rust input engine (std-only, C ABI ffi/vietflow_core.h)
├── platforms/windows/     Native Windows hook daemon (WH_KEYBOARD_LL, tray icon, Velopack updater)
├── ui/                    Qt 6 / QML configuration UI (C++ backend, chạy tiến trình riêng biệt)
├── assets/                Logo, icon PNG/ICO chuẩn High-DPI
├── docs/                  Tài liệu kiến trúc và hướng dẫn build (BUILD_CMAKE.md)
├── scripts/build.ps1      Script tự động build và đóng gói release
├── CMakeLists.txt         Unified multi-platform build orchestrator
└── Cargo.toml             Rust workspace configuration
```

- **Tiến trình Hook độc lập**: Qt UI chạy tiến trình riêng biệt khi mở cài đặt. Hook bàn phím chạy native Win32 thuần, không tải Qt runtime vào hook thread.
- **Nhận diện tiếng Anh tự động**: Danh mục 226 từ khóa lập trình và tiếng Anh phổ biến chống dính dấu tiếng Việt ngoài ý muốn.
- **Cập nhật Velopack**: Kích hoạt theo nhu cầu người dùng qua menu khay hệ thống, không chạy polling ngầm hay ép buộc cập nhật.

---

## Hướng dẫn biên dịch & Đóng gói

### 1. Yêu cầu môi trường
- Rust 1.75+ (MSVC toolchain x64)
- Qt 6.5+ (tùy chọn, để biên dịch giao diện Qt UI)
- .NET SDK & Velopack CLI (`dotnet tool install -g vpk`, tùy chọn cho đóng gói)

### 2. Lệnh thực thi

#### Cách 1: Qua CMake (Khuyên dùng cho CI/CD và đa nền tảng)
```powershell
# Cấu hình dự án
cmake -B build -S . -DCMAKE_PREFIX_PATH="C:\Qt\6.8.3\msvc2022_64"

# Chạy kiểm thử tự động
cmake --build build --target vietflow_test

# Đóng gói bản cài đặt Velopack và MSI
cmake --build build --target package_msi
```
Chi tiết các target và tùy chọn xem tại [`docs/BUILD_CMAKE.md`](file:///c:/Users/kienp/source/repos/x-unikey-1.0.4/vietflow/docs/BUILD_CMAKE.md).

#### Cách 2: Qua Script PowerShell trực tiếp
```powershell
# Kiểm thử toàn bộ workspace
cargo test --workspace

# Biên dịch Release
cargo build --workspace --release

# Đóng gói bộ cài đặt Velopack (Setup.exe và delta nupkg)
.\scripts\build.ps1 -Velopack

# Đóng gói bộ cài đặt kèm MSI
.\scripts\build.ps1 -Velopack -Msi

# Đóng gói bản tách rời Qt DLLs phục vụ phát triển
.\scripts\build.ps1 -Separate
```

---

## Giấy phép

Phát hành theo giấy phép **GNU General Public License v3.0 (GPLv3)**.
Thuật toán xử lý tiếng Việt kế thừa và tối ưu hóa từ nền tảng UniKey (tác giả Phạm Kim Long).
