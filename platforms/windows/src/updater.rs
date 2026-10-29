// VietFlow Windows - Velopack Updater Integration
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use velopack::sources::GithubSource;
use velopack::{UpdateCheck, UpdateManager};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK, MB_YESNO,
};

use crate::tray::to_wide;

pub const GITHUB_REPO_URL: &str = "https://github.com/KienPC1234/VietFlow";

static IS_CHECKING: AtomicBool = AtomicBool::new(false);

fn show_msg(text: &str, title: &str, flags: u32) -> i32 {
    let wide_text = to_wide(text);
    let wide_title = to_wide(title);
    unsafe { MessageBoxW(0 as _, wide_text.as_ptr(), wide_title.as_ptr(), flags) }
}

/// Show About dialog with app version and repository link
pub fn show_about_dialog() {
    let version = env!("CARGO_PKG_VERSION");
    let msg = format!(
        "VietFlow - Bộ gõ tiếng Việt hiện đại\n\
         Phiên bản: v{}\n\
         Bản quyền (c) 2026 VietFlow Team\n\
         Giấy phép: GNU General Public License v3.0 (GPLv3)\n\n\
         Mã nguồn mở tại:\n{}\n\n\
         Tích hợp cập nhật: Velopack (Chỉ kiểm tra khi người dùng nhấn)",
        version, GITHUB_REPO_URL
    );
    show_msg(&msg, "Thông tin VietFlow", MB_OK | MB_ICONINFORMATION);
}

struct ScopeGuard<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

/// Trigger an on-demand update check in a separate background thread.
/// Updates are never checked automatically or in a background polling loop.
pub fn check_for_updates_interactive() {
    if IS_CHECKING.swap(true, Ordering::SeqCst) {
        show_msg(
            "Hệ thống đang kiểm tra bản cập nhật. Vui lòng đợi trong giây lát...",
            "Cập nhật VietFlow",
            MB_OK | MB_ICONINFORMATION,
        );
        return;
    }

    thread::spawn(|| {
        let _guard = ScopeGuard(Some(|| {
            IS_CHECKING.store(false, Ordering::SeqCst);
        }));

        let source = GithubSource::new(GITHUB_REPO_URL, None, false);
        let um = match UpdateManager::new(source, None, None) {
            Ok(mgr) => mgr,
            Err(_) => {
                show_msg(
                    "VietFlow đang chạy ở chế độ portable / độc lập (chưa cài đặt qua gói phát hành Velopack).\n\
                     Không thể tự động cập nhật trong môi trường này.",
                    "Cập nhật VietFlow",
                    MB_OK | MB_ICONINFORMATION,
                );
                return;
            }
        };

        let current_version = um.get_current_version_as_string();

        match um.check_for_updates() {
            Ok(UpdateCheck::UpdateAvailable(info)) => {
                let target_version = &info.TargetFullRelease.Version;
                let prompt = format!(
                    "Đã có phiên bản mới: v{}\n(Phiên bản hiện tại: v{})\n\n\
                     Bạn có muốn tải về và cài đặt bản cập nhật này không?",
                    target_version, current_version
                );

                let res = show_msg(&prompt, "Cập nhật VietFlow", MB_YESNO | MB_ICONQUESTION);
                if res != IDYES {
                    return;
                }

                if let Err(e) = um.download_updates(&info, None) {
                    let err_msg = format!("Lỗi khi tải bản cập nhật: {e}");
                    show_msg(&err_msg, "Cập nhật VietFlow", MB_OK | MB_ICONERROR);
                    return;
                }

                let apply_prompt = format!(
                    "Đã tải xong bản cập nhật v{}.\n\
                     Khởi động lại VietFlow ngay bây giờ để hoàn tất cập nhật?",
                    target_version
                );

                let restart_res = show_msg(&apply_prompt, "Cập nhật VietFlow", MB_YESNO | MB_ICONQUESTION);
                if restart_res == IDYES {
                    let _ = um.apply_updates_and_restart(&info.TargetFullRelease);
                }
            }
            Ok(UpdateCheck::NoUpdateAvailable) | Ok(UpdateCheck::RemoteIsEmpty) => {
                let msg = format!(
                    "VietFlow đã là phiên bản mới nhất (v{}).",
                    current_version
                );
                show_msg(&msg, "Cập nhật VietFlow", MB_OK | MB_ICONINFORMATION);
            }
            Err(e) => {
                let err_msg = format!("Không thể kiểm tra bản cập nhật: {e}");
                show_msg(&err_msg, "Cập nhật VietFlow", MB_OK | MB_ICONERROR);
            }
        }
    });
}
