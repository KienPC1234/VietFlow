# Changelog

Mọi thay đổi đáng chú ý của VietFlow được ghi ở đây.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
phiên bản theo [Semantic Versioning](https://semver.org/lang/vi/).

## [Unreleased]

### Added

- Phím tắt riêng cho mở bảng điều khiển và mở công cụ chuyển mã (mặc định `Ctrl + Shift + F5` / `Ctrl + Shift + F9`).
- Hướng dẫn ngắn ở trang Gõ tắt và Chuyển mã.
- Không tự loại trừ cửa sổ VietFlow, để người dùng soạn macro/nguồn chuyển mã ngay trong app.
- Cấu hình riêng theo từng ứng dụng: kiểu gõ, bảng mã, cách gửi phím và gõ tắt.
- Chế độ gửi phím Unicode, scan-code, clipboard và an toàn cho app/game khó nhận ký tự.
- Gõ tắt chỉ chạy ở chế độ macro khi tiếng Việt đang tắt hoặc app bị loại trừ.
- Nhập/xuất danh sách gõ tắt từ file TSV/TXT/CSV.
- Tùy chỉnh ký tự kết thúc câu dùng để tự viết hoa.
- Nút chạy lại với quyền quản trị từ menu khay.
- Build ARM64 native và GitHub Actions cho x64/x86/ARM64.

### Fixed

- Undo bằng cách bấm đúp `[`, `]`, `{`, `}` (trước đây thành `ơơ`, `ưư`, `ƠƠ`, `ƯƯ`).
- Tone undo khôi phục đúng chuỗi gốc (`owss` → `ows`, `[ss` → `[s`), không còn `ơs`/`ưs`.
- Chỉ đối chiếu chuỗi phím gốc khi nhận diện từ tiếng Anh, tránh nhầm `mẹ` (`mej`) thành `me`.
- Danh sách tiếng Anh mặc định chỉ còn từ kỹ thuật rõ ràng; các từ ngắn dễ trùng (`test`, `reset`, `me`, `the`) không bị ép tiếng Anh, vẫn gõ được tiếng Việt. Có thể thêm/xóa trong popup.
- Khóa toàn bộ phím tắt toàn cục khi app đang focus nằm trong danh sách loại trừ; icon khay/menu vẫn đổi V/E thủ công (override tạm thời).
- Nút "Phát âm thanh" phát thử khi bật và dùng PlaySoundW/MessageBeep ổn định hơn; mặc định tắt.
- "Khởi động cùng Windows" được áp dụng ngay cả khi daemon khởi động sau khi UI đã lưu.
- Bố cục cửa sổ hẹp: tab không bị rút ngắn/ellipsis, nút phím tắt tự xuống dòng, bỏ thanh cuộn ngang ngoài, gợi ý dài xuống dòng.
- Không tự loại trừ cửa sổ VietFlow, để soạn macro/nguồn chuyển mã ngay trong app.
- Chuyển mã thật qua core C ABI, gồm chuyển byte thô TCVN3/VNI qua clipboard.
- Chế độ gửi phím "Tự động" tự chuyển sang clipboard nếu Windows chặn `SendInput`; việc khôi phục clipboard chạy nền để không chặn hook.
- Có thể tắt khóa phím tắt trong app loại trừ bằng checkbox.
- Tray override trong app loại trừ bật/tắt đúng ngay lần bấm đầu.

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
