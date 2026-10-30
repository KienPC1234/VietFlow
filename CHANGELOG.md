# Changelog

Mọi thay đổi đáng chú ý của VietFlow được ghi ở đây.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
phiên bản theo [Semantic Versioning](https://semver.org/lang/vi/).

## [Unreleased]

### Added

- Phím tắt riêng cho mở bảng điều khiển và mở công cụ chuyển mã (mặc định `Ctrl + Shift + F5` / `Ctrl + Shift + F9`).
- Hướng dẫn ngắn ở trang Gõ tắt và Chuyển mã.
- Không tự loại trừ cửa sổ VietFlow, để người dùng soạn macro/nguồn chuyển mã ngay trong app.

### Fixed

- Undo bằng cách bấm đúp `[`, `]`, `{`, `}` (trước đây thành `ơơ`, `ưư`, `ƠƠ`, `ƯƯ`).
- Tone undo khôi phục đúng chuỗi gốc (`owss` → `ows`, `[ss` → `[s`), không còn `ơs`/`ưs`.
- Mở rộng từ tiếng Anh thông dụng (`test`, `reset`, `work`, `the`...) và chỉ đối chiếu chuỗi phím gốc, tránh nhầm `mẹ` thành `me`.
- Nút "Phát âm thanh" phát thử khi bật và dùng PlaySoundW/MessageBeep ổn định hơn; mặc định tắt.
- "Khởi động cùng Windows" được áp dụng ngay cả khi daemon khởi động sau khi UI đã lưu.
- Bố cục cửa sổ hẹp: tab không bị rút ngắn/ellipsis, nút phím tắt tự xuống dòng, bỏ thanh cuộn ngang ngoài, gợi ý dài xuống dòng.
- Không tự loại trừ cửa sổ VietFlow, để soạn macro/nguồn chuyển mã ngay trong app.

## [0.1.1] - 2026-10-30

### Added

- Pipeline phát hành song song **x64 và x86**, cả hai dùng static CRT.
- `latest.json` có `downloads.x64` / `downloads.x86`; updater tự chọn đúng bản tải.
- `third_party` chuyển sang Git submodule: `imgui` (2fb554b) và `nlohmann_json` (v3.11.3).

### Changed

- README viết lại bằng tiếng Anh, gọn hơn.
- Build mặc định dùng static MSVC CRT, không cần VC++ Redistributable.
- Build script hỗ trợ `-Arch x64|x86|All` và `-DynamicCrt`.

### Fixed

- Updater đọc `latest.json` của release trước, tránh giới hạn GitHub API.
- Build x86 link đúng thư viện UI x86 (`ui_imgui/build-x86`).

## [0.1.0] - 2026-10-30

### Added

- Bộ gõ tiếng Việt Telex, VNI, VIQR, Simple Telex, Telex + VNI.
- Bảng mã Unicode, Unicode tổ hợp, TCVN3, VNI Windows.
- Gõ tắt (macro), kiểm tra chính tả, tự động khôi phục từ tiếng Anh.
- Danh sách ứng dụng loại trừ và tự chuyển [V]/[E] theo cửa sổ foreground.
- Tùy biến icon khay hệ thống, phím tắt chuyển chế độ, khởi động cùng Windows.
- Công cụ chuyển mã văn bản và clipboard.
- Kiểm tra cập nhật qua GitHub Releases, hiển thị changelog trước khi mở trang tải.

### Changed

- Giao diện chuyển sang Dear ImGui + DirectX 11, chạy ở tiến trình riêng.
- Font và icon được subset và nhúng trực tiếp vào exe.
- Bỏ Velopack runtime: exe còn ~1.2 MB, cập nhật theo cơ chế manifest.
- Toàn bộ kích thước UI scale theo DPI; hỗ trợ màn hình DPI khác nhau.

### Fixed

- UI và daemon dùng chung `%APPDATA%\VietFlow\vietflow_config.json`.
- "Khởi động cùng Windows" áp dụng ngay khi lưu, không cần khởi động lại.
- Bỏ ghi `imgui.ini` cạnh exe và sửa lệch cột ở trang chuyển mã.
