# VietFlow CMake Build & Packaging Guide

Tài liệu hướng dẫn biên dịch, kiểm thử và đóng gói phân phối VietFlow bằng CMake trên đa nền tảng.

---

## I. Yêu cầu hệ thống

| Thành phần | Phiên bản tối thiểu | Ghi chú |
| :--- | :--- | :--- |
| **CMake** | 3.21+ | Hệ thống build chính |
| **Rust Toolchain** | 1.75+ (MSVC trên Windows) | Dùng để biên dịch `core` và `platforms/windows` |
| **Qt 6** | 6.5+ (Quick, QuickControls2, Gui) | Tùy chọn, dùng để biên dịch giao diện cấu hình UI |
| **Velopack CLI** | 1.2+ (`vpk`) | Tùy chọn, dùng để đóng gói installer và MSI |
| **PowerShell** | 7.0+ (`pwsh`) hoặc Windows PowerShell | Dùng để chạy pipeline đóng gói tự động trên Windows |

---

## II. Các tùy chọn cấu hình CMake

Khi chạy `cmake -B build`, có thể truyền các cờ cấu hình sau:

| Cờ CMake | Giá trị mặc định | Mô tả |
| :--- | :--- | :--- |
| `-DVIETFLOW_DIST_DIR=<PATH>` | `${CMAKE_SOURCE_DIR}/dist` | Thư mục xuất bản phẩm phát hành |
| `-DVIETFLOW_BUILD_UI=ON\|OFF` | `ON` | Bật/tắt biên dịch giao diện Qt 6 UI |
| `-DVIETFLOW_SEPARATE_RUNTIME=ON\|OFF` | `OFF` | Xuất bản Qt DLLs tách rời thay vì nhúng vào exe |
| `-DVIETFLOW_PACK_VELOPACK=ON\|OFF` | `OFF` | Bật đóng gói tự động bộ cài Velopack |
| `-DVIETFLOW_PACK_MSI=ON\|OFF` | `OFF` | Bật tạo file cài đặt `.msi` toàn máy khi đóng gói |
| `-DCMAKE_PREFIX_PATH=<PATH>` | None | Đường dẫn đến thư mục cài đặt Qt 6 (ví dụ: `C:\Qt\6.8.3\msvc2022_64`) |

---

## III. Hướng dẫn thực thi

### 1. Cấu hình môi trường build
```powershell
cmake -B build -S . -DCMAKE_PREFIX_PATH="C:\Qt\6.8.3\msvc2022_64"
```

### 2. Biên dịch và kiểm thử tự động
```powershell
# Chạy toàn bộ 23 unit tests (Core, C ABI, KeyText, Charset codecs, Tone rules)
cmake --build build --target vietflow_test

# Chỉ biên dịch Rust Core engine (xuất vietflow_core.dll / staticlib)
cmake --build build --target vietflow_core

# Biên dịch giao diện Qt 6 UI
cmake --build build --target vietflow-ui --config Release
```

### 3. Đóng gói phân phối (Distribution)

#### A. Xuất bản phẩm portable thông thường vào `dist/`
```powershell
cmake --build build --target dist
```

#### B. Đóng gói bộ cài tự động cập nhật Velopack (`VietFlow-Setup.exe` & `releases/*.nupkg`)
```powershell
cmake --build build --target package_velopack
```

#### C. Đóng gói bộ cài mở rộng toàn máy MSI (`.msi`)
```powershell
cmake --build build --target package_msi
```

---

## IV. Kiến trúc đa nền tảng (Cross-Platform Architecture)

1. **`core/` (vietflow-core)**:
   - Thuần Rust `std`, không phụ thuộc vào bất kỳ thư viện hệ điều hành nào.
   - Xuất khẩu C ABI ổn định qua [`ffi/vietflow_core.h`](file:///c:/Users/kienp/source/repos/x-unikey-1.0.4/vietflow/ffi/vietflow_core.h).
2. **`ui/` (Qt 6 / QML)**:
   - Viết bằng Qt Quick / C++, giao tiếp với lõi xử lý hoàn toàn thông qua C ABI.
   - Chạy trên tiến trình riêng biệt, có thể chạy trên Windows, Linux (Wayland/X11) và macOS.
3. **`platforms/` (Native Host Daemons)**:
   - `platforms/windows`: Host native hook Win32 (`WH_KEYBOARD_LL`), tray icon native, bộ tự động cập nhật Velopack.
   - Tương lai bổ sung `platforms/linux` (Fcitx5 / IBus addon) và `platforms/macos` (InputMethodKit) mà không cần thay đổi `core/` hay `ui/`.

