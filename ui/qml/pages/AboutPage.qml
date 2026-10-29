// VietFlow Qt UI - About page
import QtQuick
import QtQuick.Layouts

import "../components"

PageView {
    Rectangle {
        Layout.fillWidth: true
        implicitHeight: aboutContent.implicitHeight + 40
        radius: theme.radius
        color: theme.surface
        border.width: 1
        border.color: theme.border

        ColumnLayout {
            id: aboutContent
            anchors.fill: parent
            anchors.margins: 20
            spacing: 14

            RowLayout {
                spacing: 16

                Image {
                    source: "qrc:/assets/app-logo.png"
                    sourceSize.width: 54
                    sourceSize.height: 54
                    Layout.alignment: Qt.AlignVCenter
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 3

                    Text {
                        text: "VietFlow"
                        color: theme.text
                        font.pixelSize: 21
                        font.weight: Font.Bold
                    }

                    Text {
                        text: "Phiên bản 0.1 Alpha"
                        color: "#7db2ff"
                        font.pixelSize: 12
                        font.weight: Font.DemiBold
                    }

                    Text {
                        text: "Bộ gõ tiếng Việt nhẹ, nhanh và ổn định."
                        color: theme.textDim
                        font.pixelSize: 11
                    }
                }
            }

            Rectangle {
                Layout.fillWidth: true
                Layout.preferredHeight: 1
                color: theme.border
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 8

                Text {
                    Layout.fillWidth: true
                    text: "• Hoạt động ngoại tuyến, không thu thập dữ liệu."
                    color: "#c8d0db"
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    text: "• Cấu hình được lưu trên máy tính của bạn."
                    color: "#c8d0db"
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }

                Text {
                    Layout.fillWidth: true
                    text: "• Giấy phép GNU GPL v3.0."
                    color: theme.textFaint
                    font.pixelSize: 12
                    wrapMode: Text.WordWrap
                }
            }
        }
    }

    Item { Layout.preferredHeight: 4 }

    SectionLabel { text: "PHÍM TẮT" }

    Card {
        title: "Mở cửa sổ cấu hình"
        description: "Dùng từ bất kỳ ứng dụng nào"

        KeyCap { label: "Ctrl" }
        Text {
            text: "+"
            color: theme.textFaint
            font.pixelSize: 11
            Layout.alignment: Qt.AlignVCenter
        }
        KeyCap { label: "Shift" }
        Text {
            text: "+"
            color: theme.textFaint
            font.pixelSize: 11
            Layout.alignment: Qt.AlignVCenter
        }
        KeyCap { label: "F5" }
    }

    Card {
        title: "Mở công cụ chuyển mã"
        description: "Chuyển nhanh nội dung trong Clipboard"

        KeyCap { label: "Ctrl" }
        Text {
            text: "+"
            color: theme.textFaint
            font.pixelSize: 11
            Layout.alignment: Qt.AlignVCenter
        }
        KeyCap { label: "Shift" }
        Text {
            text: "+"
            color: theme.textFaint
            font.pixelSize: 11
            Layout.alignment: Qt.AlignVCenter
        }
        KeyCap { label: "F9" }
    }
}
