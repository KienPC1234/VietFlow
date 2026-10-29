// VietFlow Qt UI - Main window
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import "components"
import "pages"

ApplicationWindow {
    id: root

    width: 900
    height: 600
    minimumWidth: 800
    minimumHeight: 540
    visible: true
    title: "VietFlow - Bảng điều khiển bộ gõ tiếng Việt"
    color: theme.bg

    property int pageIndex: initialPage

    readonly property var pageMeta: [
        {
            title: "Tổng quan",
            subtitle: "Thiết lập nhanh kiểu gõ và bảng mã."
        },
        {
            title: "Bộ gõ & Chính tả",
            subtitle: "Tinh chỉnh cách bỏ dấu và tự sửa lỗi."
        },
        {
            title: "Bảng gõ tắt",
            subtitle: "Thay thế cụm từ khi gõ."
        },
        {
            title: "Game Mode",
            subtitle: "Tự chuyển sang English khi chơi game."
        },
        {
            title: "Chuyển mã & Tiện ích",
            subtitle: "Chuyển đổi bảng mã văn bản và Clipboard."
        },
        {
            title: "Nâng cao",
            subtitle: "Danh sách từ và tùy chọn ít dùng."
        },
        {
            title: "Giới thiệu",
            subtitle: "Phiên bản và giấy phép."
        }
    ]

    RowLayout {
        anchors.fill: parent
        spacing: 0

        Sidebar {
            Layout.fillHeight: true
            Layout.preferredWidth: 216
            currentIndex: root.pageIndex
            onNavigate: (index) => root.pageIndex = index
        }

        ColumnLayout {
            Layout.fillWidth: true
            Layout.fillHeight: true
            spacing: 0

            // Header
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 78

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 26
                    anchors.rightMargin: 26
                    spacing: 16

                    ColumnLayout {
                        Layout.fillWidth: true
                        Layout.alignment: Qt.AlignVCenter
                        spacing: 3

                        Text {
                            Layout.fillWidth: true
                            text: root.pageMeta[root.pageIndex].title
                            color: theme.text
                            font.pixelSize: 20
                            font.weight: Font.Bold
                            elide: Text.ElideRight
                        }

                        Text {
                            Layout.fillWidth: true
                            text: root.pageMeta[root.pageIndex].subtitle
                            color: theme.textDim
                            font.pixelSize: 11
                            elide: Text.ElideRight
                        }
                    }

                    StatusPill {
                        Layout.alignment: Qt.AlignVCenter
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: theme.border
            }

            // Pages
            StackLayout {
                Layout.fillWidth: true
                Layout.fillHeight: true
                currentIndex: root.pageIndex

                OverviewPage {}
                TypingPage {}
                MacroPage {}
                GamePage {}
                ConverterPage {}
                AdvancedPage {}
                AboutPage {}
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: theme.border
            }

            // Footer
            Item {
                Layout.fillWidth: true
                Layout.preferredHeight: 58

                RowLayout {
                    anchors.fill: parent
                    anchors.leftMargin: 26
                    anchors.rightMargin: 26
                    spacing: 10

                    Text {
                        Layout.fillWidth: true
                        text: backend.statusText
                        color: theme.textFaint
                        font.pixelSize: 11
                        elide: Text.ElideRight
                        verticalAlignment: Text.AlignVCenter
                    }

                    GhostButton {
                        text: "Mặc định"
                        onClicked: backend.resetDefaults()
                    }

                    GhostButton {
                        text: "Đóng"
                        onClicked: root.close()
                    }

                    PrimaryButton {
                        text: "Lưu"
                        onClicked: backend.save()
                    }
                }
            }
        }
    }
}
