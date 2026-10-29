// VietFlow Qt UI - Charset converter page
import QtQuick
import QtQuick.Layouts

import "../components"

PageView {
    id: root

    property int srcIndex: 0
    property int dstIndex: 2
    property string clipboardStatus: ""
    property string textStatus: ""

    readonly property var charsets: [
        "Unicode",
        "Unicode tổ hợp",
        "TCVN3 (ABC)",
        "VNI Windows"
    ]

    SectionLabel { text: "CHUYỂN MÃ CLIPBOARD" }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: clipboardContent.implicitHeight + 36
        radius: theme.radius
        color: theme.surface
        border.width: 1
        border.color: theme.border

        ColumnLayout {
            id: clipboardContent
            anchors.fill: parent
            anchors.margins: 18
            spacing: 12

            Text {
                Layout.fillWidth: true
                text: "Đọc Clipboard, chuyển bảng mã rồi ghi lại."
                color: theme.textDim
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 16

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    Text {
                        text: "Mã nguồn"
                        color: theme.textDim
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }

                    Select {
                        Layout.fillWidth: true
                        items: root.charsets
                        currentIndex: root.srcIndex
                        onActivated: (index) => root.srcIndex = index
                    }
                }

                ColumnLayout {
                    Layout.fillWidth: true
                    spacing: 5

                    Text {
                        text: "Mã đích"
                        color: theme.textDim
                        font.pixelSize: 11
                        font.weight: Font.DemiBold
                    }

                    Select {
                        Layout.fillWidth: true
                        items: root.charsets
                        currentIndex: root.dstIndex
                        onActivated: (index) => root.dstIndex = index
                    }
                }
            }

            RowLayout {
                spacing: 12

                PrimaryButton {
                    text: "Chuyển mã Clipboard"
                    onClicked: root.clipboardStatus = engine.convertClipboard(root.srcIndex, root.dstIndex)
                }

                Text {
                    text: root.clipboardStatus
                    color: theme.success
                    font.pixelSize: 11
                    font.weight: Font.Medium
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }

    Item { Layout.preferredHeight: 6 }

    SectionLabel { text: "CHUYỂN MÃ VĂN BẢN" }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: textContent.implicitHeight + 36
        radius: theme.radius
        color: theme.surface
        border.width: 1
        border.color: theme.border

        ColumnLayout {
            id: textContent
            anchors.fill: parent
            anchors.margins: 18
            spacing: 12

            AppTextArea {
                id: convertArea
                Layout.fillWidth: true
                Layout.preferredHeight: 120
                placeholderText: "Nhập hoặc dán nội dung cần chuyển mã vào đây..."
            }

            RowLayout {
                spacing: 12

                PrimaryButton {
                    text: "Thực hiện chuyển đổi"
                    onClicked: {
                        convertArea.text = engine.convert(convertArea.text, root.srcIndex, root.dstIndex);
                        root.textStatus = "Đã chuyển đổi văn bản.";
                    }
                }

                Text {
                    text: root.textStatus
                    visible: root.textStatus.length > 0
                    color: theme.success
                    font.pixelSize: 11
                    verticalAlignment: Text.AlignVCenter
                }
            }
        }
    }
}
