// VietFlow Qt UI - Macro page
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import "../components"

Item {
    id: root

    function submitMacro() {
        if (macroKey.text.trim().length === 0 || macroValue.text.length === 0) {
            return;
        }
        backend.addMacro(macroKey.text, macroValue.text);
        macroKey.text = "";
        macroValue.text = "";
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 8

        Card {
            title: "Bật gõ tắt (Macro)"
            description: "Thay từ viết tắt bằng cụm từ đầy đủ."

            Toggle {
                checked: backend.options.enableMacro
                onToggled: backend.setOption("enableMacro", !backend.options.enableMacro)
            }
        }

        Card {
            title: "Gõ tắt khi ở chế độ English"
            description: "Vẫn thay thế macro khi đang tắt tiếng Việt."

            Toggle {
                checked: backend.options.allowMacroWhenDisabled
                onToggled: backend.setOption("allowMacroWhenDisabled", !backend.options.allowMacroWhenDisabled)
            }
        }

        HintBanner {
            label: "Có thể dùng"
            message: "@date · @time · @datetime · @clipboard · @tab · @newline"
        }

        Rectangle {
            Layout.fillWidth: true
            Layout.fillHeight: true
            radius: theme.radius
            color: theme.surface
            border.width: 1
            border.color: theme.border
            clip: true

            ColumnLayout {
                anchors.fill: parent
                spacing: 0

                Rectangle {
                    Layout.fillWidth: true
                    Layout.preferredHeight: 32
                    color: theme.surfaceAlt

                    RowLayout {
                        anchors.fill: parent
                        anchors.leftMargin: 20
                        anchors.rightMargin: 14
                        spacing: 12

                        Item {
                            Layout.preferredWidth: 170
                            Layout.fillHeight: true

                            Text {
                                anchors.fill: parent
                                text: "TỪ TẮT"
                                color: theme.textFaint
                                font.pixelSize: 10
                                font.weight: Font.DemiBold
                                font.letterSpacing: 0.8
                                verticalAlignment: Text.AlignVCenter
                            }
                        }

                        Text {
                            Layout.fillWidth: true
                            text: "CỤM TỪ THAY THẾ"
                            color: theme.textFaint
                            font.pixelSize: 10
                            font.weight: Font.DemiBold
                            font.letterSpacing: 0.8
                            verticalAlignment: Text.AlignVCenter
                        }

                        Item { Layout.preferredWidth: 26 }
                    }
                }

                Item {
                    Layout.fillWidth: true
                    Layout.fillHeight: true

                    ListView {
                        id: macroList
                        anchors.fill: parent
                        anchors.margins: 6
                        clip: true
                        spacing: 2
                        model: backend.macros

                        delegate: Rectangle {
                            width: ListView.view.width
                            height: 38
                            radius: 6
                            color: rowHover.hovered
                                   ? theme.surfaceHover
                                   : (index % 2 === 0 ? "transparent" : theme.surfaceAlt)

                            HoverHandler { id: rowHover }

                            RowLayout {
                                anchors.fill: parent
                                anchors.leftMargin: 14
                                anchors.rightMargin: 2
                                spacing: 12

                                Item {
                                    Layout.preferredWidth: 170
                                    Layout.fillHeight: true

                                    Text {
                                        anchors.fill: parent
                                        text: model.trigger
                                        color: "#7db2ff"
                                        font.pixelSize: 12
                                        font.weight: Font.DemiBold
                                        elide: Text.ElideRight
                                        verticalAlignment: Text.AlignVCenter
                                    }
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: model.expansion
                                    color: theme.text
                                    font.pixelSize: 12
                                    elide: Text.ElideRight
                                    verticalAlignment: Text.AlignVCenter
                                }

                                IconButton {
                                    onClicked: backend.removeMacro(index)
                                }
                            }
                        }
                    }

                    Text {
                        anchors.centerIn: parent
                        visible: macroList.count === 0
                        text: "Chưa có macro nào. Thêm từ viết tắt ở ô bên dưới."
                        color: theme.textFaint
                        font.pixelSize: 12
                    }
                }
            }
        }

        RowLayout {
            Layout.fillWidth: true
            spacing: 10

            AppTextField {
                id: macroKey
                Layout.preferredWidth: 170
                placeholderText: "Từ viết tắt (vd: vn)"
                onAccepted: root.submitMacro()
            }

            AppTextField {
                id: macroValue
                Layout.fillWidth: true
                placeholderText: "Cụm từ thay thế (vd: Việt Nam hoặc @date)"
                onAccepted: root.submitMacro()
            }

            PrimaryButton {
                text: "Thêm / Sửa"
                onClicked: root.submitMacro()
            }
        }
    }
}
