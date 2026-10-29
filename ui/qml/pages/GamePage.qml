// VietFlow Qt UI - Game Mode page
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

import "../components"

Item {
    id: root

    function submitApp() {
        if (appInput.text.trim().length === 0) {
            return;
        }
        backend.addExcludedApp(appInput.text);
        appInput.text = "";
    }

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 20
        spacing: 8

        HintBanner {
            label: "Game Mode"
            message: "Tự chuyển sang English khi các ứng dụng dưới đây đang chạy."
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

                        Text {
                            Layout.fillWidth: true
                            text: "ỨNG DỤNG"
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
                        id: appList
                        anchors.fill: parent
                        anchors.margins: 6
                        clip: true
                        spacing: 2
                        model: backend.excludedApps

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
                                spacing: 10

                                Image {
                                    source: "qrc:/assets/icons/game-dim.svg"
                                    sourceSize.width: 16
                                    sourceSize.height: 16
                                    Layout.alignment: Qt.AlignVCenter
                                }

                                Text {
                                    Layout.fillWidth: true
                                    text: model.name
                                    color: theme.text
                                    font.pixelSize: 12
                                    elide: Text.ElideRight
                                    verticalAlignment: Text.AlignVCenter
                                }

                                IconButton {
                                    onClicked: backend.removeExcludedApp(index)
                                }
                            }
                        }
                    }

                    Text {
                        anchors.centerIn: parent
                        visible: appList.count === 0
                        text: "Chưa có ứng dụng nào trong danh sách loại trừ."
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
                id: appInput
                Layout.fillWidth: true
                placeholderText: "Tên ứng dụng (vd: cs2.exe)"
                onAccepted: root.submitApp()
            }

            PrimaryButton {
                text: "Thêm ứng dụng"
                onClicked: root.submitApp()
            }
        }
    }
}
