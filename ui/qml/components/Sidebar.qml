// VietFlow Qt UI - Left navigation sidebar
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    property int currentIndex: 0
    signal navigate(int index)

    color: theme.sidebar

    ColumnLayout {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 4

        RowLayout {
            Layout.fillWidth: true
            Layout.preferredHeight: 48
            spacing: 10

            Image {
                source: "qrc:/assets/app-logo.png"
                sourceSize.width: 34
                sourceSize.height: 34
                Layout.alignment: Qt.AlignVCenter
            }

            ColumnLayout {
                Layout.fillWidth: true
                spacing: 0

                Text {
                    text: "VietFlow"
                    color: theme.text
                    font.pixelSize: 17
                    font.weight: Font.DemiBold
                }

                Text {
                    text: "Bộ gõ tiếng Việt"
                    color: theme.textFaint
                    font.pixelSize: 10
                }
            }
        }

        Item { Layout.preferredHeight: 10 }

        SectionLabel { text: "ĐIỀU HƯỚNG" }

        Repeater {
            model: [
                {
                    label: "Tổng quan",
                    icon: "qrc:/assets/icons/dashboard-dim.svg",
                    iconActive: "qrc:/assets/icons/dashboard-active.svg"
                },
                {
                    label: "Bộ gõ & Chính tả",
                    icon: "qrc:/assets/icons/spellcheck-dim.svg",
                    iconActive: "qrc:/assets/icons/spellcheck-active.svg"
                },
                {
                    label: "Bảng gõ tắt",
                    icon: "qrc:/assets/icons/macro-dim.svg",
                    iconActive: "qrc:/assets/icons/macro-active.svg"
                },
                {
                    label: "Game Mode",
                    icon: "qrc:/assets/icons/game-dim.svg",
                    iconActive: "qrc:/assets/icons/game-active.svg"
                },
                {
                    label: "Chuyển mã",
                    icon: "qrc:/assets/icons/sparkles-dim.svg",
                    iconActive: "qrc:/assets/icons/sparkles-active.svg"
                },
                {
                    label: "Nâng cao",
                    icon: "qrc:/assets/icons/settings-dim.svg",
                    iconActive: "qrc:/assets/icons/settings-active.svg"
                },
                {
                    label: "Giới thiệu",
                    icon: "qrc:/assets/icons/keyboard-dim.svg",
                    iconActive: "qrc:/assets/icons/keyboard-active.svg"
                }
            ]

            delegate: NavButton {
                Layout.fillWidth: true
                label: modelData.label
                icon: modelData.icon
                iconActive: modelData.iconActive
                active: root.currentIndex === index
                onClicked: root.navigate(index)
            }
        }

        Item { Layout.fillHeight: true }

        Rectangle {
            Layout.fillWidth: true
            implicitHeight: 58
            radius: 8
            color: theme.surface
            border.width: 1
            border.color: theme.border

            ColumnLayout {
                anchors.fill: parent
                anchors.margins: 10
                spacing: 2

                Text {
                    text: "VietFlow 0.1 Alpha"
                    color: theme.text
                    font.pixelSize: 11
                    font.weight: Font.DemiBold
                }

                Text {
                    text: "GPLv3"
                    color: theme.textFaint
                    font.pixelSize: 10
                }
            }
        }
    }
}
