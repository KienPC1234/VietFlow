// VietFlow Qt UI - Overview page
import QtQuick
import QtQuick.Layouts

import "../components"

PageView {
    SectionLabel { text: "THIẾT LẬP BỘ GÕ" }

    Card {
        title: "Kiểu gõ tiếng Việt"
        description: "Telex, Simple Telex, VNI hoặc VIQR"

        Select {
            Layout.preferredWidth: 230
            items: ["Telex", "Simple Telex", "VNI", "VIQR"]
            currentIndex: backend.options.method
            onActivated: (index) => backend.setOption("method", index)
        }
    }

    Card {
        title: "Bảng mã ký tự"
        description: "Unicode (NFC), tổ hợp, TCVN3, VNI Windows"

        Select {
            Layout.preferredWidth: 230
            items: ["Unicode dựng sẵn (NFC)", "Unicode tổ hợp", "TCVN3 (ABC)", "VNI Windows"]
            currentIndex: backend.options.charset
            onActivated: (index) => backend.setOption("charset", index)
        }
    }

    Card {
        title: "Phím chuyển đổi V / E"
        description: "Chuyển nhanh giữa Tiếng Việt và English"

        Select {
            Layout.preferredWidth: 230
            items: ["Ctrl + Shift", "Alt + Z", "Ctrl + Space", "Alt + Shift"]
            currentIndex: backend.options.hotkeyIndex
            onActivated: (index) => backend.setOption("hotkeyIndex", index)
        }
    }

    Item { Layout.preferredHeight: 6 }

    SectionLabel { text: "HỆ THỐNG & KHỞI ĐỘNG" }

    Card {
        title: "Khởi động cùng Windows"
        description: "Tự chạy khi đăng nhập Windows"

        Toggle {
            checked: backend.options.startWithWindows
            onToggled: backend.setOption("startWithWindows", !backend.options.startWithWindows)
        }
    }

    Card {
        title: "Âm thanh thông báo"
        description: "Bíp nhẹ khi chuyển chế độ"

        Toggle {
            checked: backend.options.notificationSound
            onToggled: backend.setOption("notificationSound", !backend.options.notificationSound)
        }
    }
}
