# Changelog

Mọi thay đổi đáng chú ý của VietFlow được ghi ở đây.
Định dạng dựa trên [Keep a Changelog](https://keepachangelog.com/vi/1.1.0/),
phiên bản theo [Semantic Versioning](https://semver.org/lang/vi/).

## [0.1.2] - 2026-11-01

### Added

- Phản hồi 0ms không độ trễ (Native Passthrough): Các phím gõ đơn không biến đổi âm tiết (`is_plain_passthrough`) được chuyển tiếp trực tiếp bằng `CallNextHookEx`, không nuốt phím và không phát SendInput ảo.
- Hỗ trợ di chuyển mượt mà trong game (Hardware Key Auto-Repeat): Quản lý `held_vk` cho phép giữ phím (WASD) di chuyển liên tục trong game (Roblox, CS2, Valorant) theo đúng tần số lặp phần cứng của bàn phím.
- Nhận diện game/ứng dụng trong container UWP / Windows Store / Xbox GDK: Tự động phân giải tiến trình vỏ `ApplicationFrameHost.exe` thông qua duyệt cây cửa sổ con (`EnumChildWindows`) để trích xuất chính xác PID và file thực thi gốc (ví dụ: `RobloxPlayerBeta.exe`).
- Khớp tiền tố thông minh cho danh sách loại trừ (`app_name_matches`): Người dùng chỉ cần thêm `roblox` thì mọi biến thể tiến trình (`RobloxPlayerBeta.exe`, `RobloxStudioBeta.exe`, `roblox.exe`) đều được tự động nhận diện và loại trừ.
- Bổ sung nút bấm nhanh preset Roblox vào giao diện Cài đặt (trang Loại trừ).
- Thiết kế lại phần hiển thị và giải thích chế độ TSF trong giao diện Cài đặt: Diễn giải rõ ràng lợi ích gõ trực tiếp không dùng Backspace (trị dứt điểm giật con trỏ trong Excel/Word/trình duyệt), nút đăng ký/hủy đăng ký trực quan kèm nhãn trạng thái thời gian thực.
- Bổ sung bảng so sánh tính năng toàn diện giữa VietFlow, EVKey, OpenKey và UniKey 4.3 trong tài liệu `README.md`.
- Biểu tượng thế hệ mới: vector full-bleed đa độ phân giải (16 đến 256 px) cho chế độ thường (`VietFlow.ico`) và biểu tượng bàn phím TSF (`VietFlow-TSF.ico`) cho chế độ TSF IME, hiển thị sắc nét chuẩn DPI theo `SM_CXSMICON`.
- Chế độ TSF vẫn giữ low-level hook nhẹ để phím tắt, danh sách loại trừ và override tạm thời tiếp tục hoạt động; phần gõ phím được nhường cho TSF nên không xử lý trùng.
- Nút "Cài lại TSF TIP" trong cài đặt chế độ TSF IME để tự động unregister và re-register TIP COM server sạch sẽ vào hệ thống.
- Xử lý bộ gõ TSF theo kiến trúc Linux IME (IBus/Fcitx): tách biệt trạng thái preedit qua `current_preedit_text()`, tự động mở rộng macro và khôi phục tiếng Anh (`finish_composition_on_break`) ngay tại ranh giới từ (Space, Enter, Tab, ký tự ngắt từ) thông qua action `ReplaceAndEndComposition` trước khi kết thúc composition.
- Đồng bộ `enable_macro` qua IPC bộ nhớ dùng chung `TsfSharedConfig`.
- Phím tắt riêng cho mở bảng điều khiển và mở công cụ chuyển mã (mặc định `Ctrl + Shift + F5` / `Ctrl + Shift + F9`).
- Icon khay tùy chỉnh cho từng trạng thái V/E: chọn file `.ico` hoặc `.png` trong tab Giao diện; icon riêng thay cho chữ V/E mặc định.
- Macro `@clipboard` lấy nội dung clipboard qua cache an toàn của daemon; TSF cũng hỗ trợ khi macro được dùng trong app TSF.
- TSF đọc thêm macro, custom keymap, phụ âm ngoại lai, danh sách từ tiếng Anh, timeout và profile theo app từ file cấu hình.
- Hướng dẫn ngắn ở trang Gõ tắt và Chuyển mã.
- Không tự loại trừ cửa sổ VietFlow, để người dùng soạn macro/nguồn chuyển mã ngay trong app.
- Cấu hình riêng theo từng ứng dụng: kiểu gõ, bảng mã, cách gửi phím và gõ tắt.
- Chế độ gửi phím Unicode, scan-code và an toàn cho app/game khó nhận ký tự.
- Gõ tắt chỉ chạy ở chế độ macro khi tiếng Việt đang tắt hoặc app bị loại trừ.
- Nhập/xuất danh sách gõ tắt từ file TSV/TXT/CSV.
- Tùy chỉnh ký tự kết thúc câu dùng để tự viết hoa.
- Nút chạy lại với quyền quản trị từ menu khay.
- Build ARM64 native và GitHub Actions cho x64/x86/ARM64.

### Fixed

- Sửa lỗi tràn nhãn và đè giao diện: Nới rộng khoảng cách cột nhãn icon riêng (`Tiếng Anh [E]:`, `Tiếng Việt [V]:`) trong tab Giao diện, tránh việc text box bị đè lên nhãn.
- Undo bằng cách bấm đúp `[`, `]`, `{`, `}` (trước đây thành `ơơ`, `ưư`, `ƠƠ`, `ƯƯ`).
- Tone undo khôi phục đúng chuỗi gốc (`owss` → `ows`, `[ss` → `[s`), không còn `ơs`/`ưs`.
- Chỉ đối chiếu chuỗi phím gốc khi nhận diện từ tiếng Anh, tránh nhầm `mẹ` (`mej`) thành `me`.
- Danh sách tiếng Anh mặc định chỉ còn từ kỹ thuật rõ ràng; các từ ngắn dễ trùng (`test`, `reset`, `me`, `the`) không bị ép tiếng Anh, vẫn gõ được tiếng Việt. Có thể thêm/xóa trong popup.
- Khóa toàn bộ phím tắt toàn cục khi app đang focus nằm trong danh sách loại trừ; icon khay/menu vẫn đổi V/E thủ công (override tạm thời).
- Nút "Phát âm thanh" phát thử khi bật và dùng PlaySoundW/MessageBeep ổn định hơn; mặc định tắt.
- "Khởi động cùng Windows" được áp dụng ngay cả khi daemon khởi động sau khi UI đã lưu.
- Bố cục cửa sổ hẹp: tab không bị rút ngắn/ellipsis, nút phím tắt tự xuống dòng, bỏ thanh cuộn ngang ngoài, gợi ý dài xuống dòng.
- Chuyển mã thật qua core C ABI, gồm chuyển byte thô TCVN3/VNI qua clipboard.
- Chế độ gửi phím "Tự động" fallback sang gửi chậm nếu Windows chặn `SendInput`, không tự chuyển Clipboard.
- Có thể tắt khóa phím tắt trong app loại trừ bằng checkbox.
- Tray override trong app loại trừ bật/tắt đúng ngay lần bấm đầu.
- Hộp thoại kiểm tra cập nhật luôn hiện trên cửa sổ cài đặt thay vì nằm phía sau.
- Sửa rò GDI icon khay khi lưu cấu hình nhiều lần; sau 20 lần reload, GDI objects giữ nguyên 12 thay vì tăng lên 132.
- Chỉ tạo lại icon khay khi cấu hình icon thay đổi.
- Icon khay V/E dùng font riêng `VietFlow Tray` (~2,2 KB): V lấy từ **Roboto SemiBold**, E lấy từ **JetBrains Mono SemiBold**, cân cùng chiều cao thị giác; bỏ GDI+ để giảm khởi tạo và rò tài nguyên, đồng thời giữ nét sắc ở mọi DPI.
- Ổn định âm thanh thông báo V/E: nhúng sẵn chime IMA ADPCM ngắn (~0,16 giây, ~2,1 KB), âm lượng thấp, tần số mềm; phát bằng một worker tuần tự, gộp các lần bấm nhanh, có fallback `%SystemRoot%\Media`/alias/beep và bỏ việc UI + daemon cùng phát một tiếng.
- UI giải phóng D3D11 device/swap chain khi minimize và tạo lại khi mở, giảm khoảng 10 MB working set / 6,5 MB private bytes.
- Bỏ chế độ gửi phím Clipboard; cấu hình cũ dùng Clipboard tự chuyển về Tự động, tránh lỗi và nhận diện nhầm.
- Tự tắt tiếng Việt trong cửa sổ ANSI-only như VBA IDE; có checkbox để tắt tính năng này nếu app vẫn nhận tiếng Việt.
- Sửa banner cảnh báo IME bị cắt chân chữ và hiện scrollbar; căn lại header "Kiểu gõ tự định nghĩa".
- Gõ tắt mặc định tắt; UI chỉ bật checkbox phụ khi người dùng chủ động bật gõ tắt.
- Cảnh báo bộ gõ tiếng Việt khác chuyển từ banner trong UI sang hộp thoại khi khởi động daemon.
- Giữ trạng thái tạm bật tiếng Việt khi bấm icon khay từ app loại trừ; trở về app khác sẽ tự tắt lại.
- Chuyển mã Unicode tổ hợp (NFD) đúng cả chiều đọc/ghi, gồm circumflex/breve/horn + dấu thanh; bỏ dấu hoạt động với văn bản NFD.
- UI: sửa ID trùng giữa các dòng profile app, màu icon khay bị cũ sau reset, ghi quá 1 byte khi đổi đường dẫn/tên app, lower-case UTF-8 và chỉ số combo cấu hình lỗi.
- UI: kiểm tra lỗi tạo render target D3D11 thay vì tiếp tục vẽ vào null.
- Icon ứng dụng bị đen: tạo lại `VietFlow-Normal.ico` và `VietFlow-TSF.ico` trực tiếp từ SVG gốc, đủ frame 16–256 px; app x64/x86 và `dist` đã dùng icon đã sửa.
- UI: rút gọn nhãn và thêm tooltip giải thích cho phím tắt trong app loại trừ, cách gửi phím, TSF và các tùy chọn dễ nhầm; nội dung chọn trong bảng profile ngắn gọn hơn.
- UI: tự reload cấu hình khi daemon đổi V/E/tray, tránh ghi đè thay đổi; nút khôi phục mặc định giữ đúng danh sách app và macro gốc.
- Menu tray hiển thị đúng phím tắt cài đặt/chuyển mã; phím tắt chuyển mã mở đúng cửa sổ chuyển mã ngay cả khi bảng điều khiển đang mở.
- Tối ưu hot path: tra macro không cấp phát `String` cho trigger ASCII thường; UI kiểm tra config mỗi 250 ms và trạng thái TSF mỗi 1 giây thay vì mỗi frame; TSF cache cấu hình/profile và chỉ đọc clipboard khi macro có `@clipboard`.

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
