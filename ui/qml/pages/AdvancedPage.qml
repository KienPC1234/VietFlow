// VietFlow Qt UI - Advanced settings page
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import "../components"

PageView {
    id: root

    function keywordCount() {
        var lines = keywordsArea.text.split(/\r?\n/);
        var count = 0;
        for (var i = 0; i < lines.length; ++i) {
            if (lines[i].trim().length > 0) {
                count += 1;
            }
        }
        return count;
    }

    SectionLabel { text: "DANH SÁCH TỪ" }

    Rectangle {
        Layout.fillWidth: true
        Layout.preferredHeight: keywordContent.implicitHeight + 36
        radius: theme.radius
        color: theme.surface
        border.width: 1
        border.color: theme.border

        ColumnLayout {
            id: keywordContent
            anchors.fill: parent
            anchors.margins: 18
            spacing: 10

            Text {
                text: "Từ tiếng Anh / từ ngoại lai"
                color: theme.text
                font.pixelSize: 13
                font.weight: Font.DemiBold
            }

            Text {
                Layout.fillWidth: true
                text: "Mỗi dòng một từ. Các từ này sẽ không bị bỏ dấu tiếng Việt."
                color: theme.textDim
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }

            AppTextArea {
                id: keywordsArea
                Layout.fillWidth: true
                Layout.preferredHeight: 150
                placeholderText: "Mỗi dòng một từ (để trống để dùng danh sách mặc định)"
                onTextChanged: backend.setOption("englishKeywords", text)
                Component.onCompleted: text = backend.options.englishKeywords
            }

            RowLayout {
                Layout.fillWidth: true
                spacing: 10

                Text {
                    Layout.fillWidth: true
                    text: root.keywordCount() === 0
                          ? "Đang dùng danh sách mặc định"
                          : root.keywordCount() + " từ tùy chỉnh"
                    color: theme.textFaint
                    font.pixelSize: 11
                    verticalAlignment: Text.AlignVCenter
                }

                GhostButton {
                    text: "Khôi phục mặc định"
                    onClicked: {
                        keywordsArea.text = "";
                        backend.setOption("englishKeywords", "");
                    }
                }

                PrimaryButton {
                    text: "Lưu danh sách"
                    onClicked: backend.save()
                }
            }
        }

        Connections {
            target: backend

            function onOptionsChanged() {
                if (!keywordsArea.activeFocus
                        && keywordsArea.text !== backend.options.englishKeywords) {
                    keywordsArea.text = backend.options.englishKeywords;
                }
                if (!initialsField.activeFocus
                        && initialsField.text !== backend.options.foreignInitials) {
                    initialsField.text = backend.options.foreignInitials;
                }
            }
        }
    }

    Card {
        title: "Phụ âm đầu nước ngoài"
        description: "Các chữ cái này sẽ không nhận dấu (mặc định fjwz)."

        AppTextField {
            id: initialsField
            Layout.preferredWidth: 150
            placeholderText: "fjwz"
            validator: RegularExpressionValidator { regularExpression: /^[A-Za-z]{0,26}$/ }
            onTextChanged: backend.setOption("foreignInitials", text)
            Component.onCompleted: text = backend.options.foreignInitials
        }
    }

    Item { Layout.preferredHeight: 4 }

    SectionLabel { text: "HỆ THỐNG" }

    Card {
        title: "Tệp cấu hình"
        description: "Mở thư mục chứa tệp cấu hình."

        Text {
            Layout.preferredWidth: 260
            text: backend.configPath
            color: theme.textFaint
            font.pixelSize: 10
            elide: Text.ElideMiddle
            verticalAlignment: Text.AlignVCenter
        }

        GhostButton {
            text: "Mở thư mục"
            onClicked: backend.openConfigFolder()
        }
    }
}
