// VietFlow Qt UI - Typing and spelling page
import QtQuick
import QtQuick.Layouts

import "../components"

PageView {
    SectionLabel { text: "TÍNH NĂNG THÔNG MINH" }

    Card {
        title: "Kiểm tra chính tả tiếng Việt"
        description: "Bỏ dấu theo cấu trúc từ tiếng Việt hợp lệ"

        Toggle {
            checked: backend.options.spellCheck
            onToggled: backend.setOption("spellCheck", !backend.options.spellCheck)
        }
    }

    Card {
        title: "Tự động khôi phục từ tiếng Anh"
        description: "Trả lại từ tiếng Anh khi gõ nhầm dấu"

        Toggle {
            checked: backend.options.autoRestore
            onToggled: backend.setOption("autoRestore", !backend.options.autoRestore)
        }
    }

    Card {
        title: "Đặt dấu thanh kiểu mới"
        description: "Dùng oà, uỳ, oè thay cho òa, ùy, òe"

        Toggle {
            checked: backend.options.modernStyle
            onToggled: backend.setOption("modernStyle", !backend.options.modernStyle)
        }
    }

    Card {
        title: "Cho phép bỏ dấu tự do"
        description: "Bỏ dấu ở bất kỳ vị trí trong từ"

        Toggle {
            checked: backend.options.freeMarking
            onToggled: backend.setOption("freeMarking", !backend.options.freeMarking)
        }
    }

    Card {
        title: "Tự sửa dính Shift chữ thứ hai"
        description: "Sửa lỗi VIệt → Việt khi gõ nhanh"

        Toggle {
            checked: backend.options.fixShiftLag
            onToggled: backend.setOption("fixShiftLag", !backend.options.fixShiftLag)
        }
    }

    Card {
        title: "Tốc ký phụ âm đầu"
        description: "f → ph, j → gi, w → qu"

        Toggle {
            checked: backend.options.consonantShortcuts
            onToggled: backend.setOption("consonantShortcuts", !backend.options.consonantShortcuts)
        }
    }

    Card {
        title: "Tự động viết hoa sau dấu câu"
        description: "Viết hoa đầu câu sau . ! ?"

        Toggle {
            checked: backend.options.autoCapitalize
            onToggled: backend.setOption("autoCapitalize", !backend.options.autoCapitalize)
        }
    }

    Card {
        title: "Dùng Clipboard cho bảng mã đặc biệt"
        description: "Tương thích ứng dụng cũ không nhận Unicode"

        Toggle {
            checked: backend.options.useClipboard
            onToggled: backend.setOption("useClipboard", !backend.options.useClipboard)
        }
    }
}
