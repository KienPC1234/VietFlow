// VietFlow Qt UI - Live V/E status pill
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    readonly property bool live: backend.options.enabled === true

    implicitWidth: 202
    implicitHeight: 46
    radius: 23
    color: theme.surface
    border.width: 1
    border.color: root.live ? "#2f4a3f" : theme.border
    Behavior on border.color { ColorAnimation { duration: 150 } }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 9
        spacing: 10

        Rectangle {
            width: 8
            height: 8
            radius: 4
            color: root.live ? theme.success : theme.textFaint
            Layout.alignment: Qt.AlignVCenter
            Behavior on color { ColorAnimation { duration: 150 } }
        }

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 0

            Text {
                text: root.live ? "Tiếng Việt" : "English"
                color: theme.text
                font.pixelSize: 12
                font.weight: Font.DemiBold
            }

            Text {
                text: root.live ? "Đang bật" : "Đang tắt"
                color: root.live ? theme.success : theme.textFaint
                font.pixelSize: 10
            }
        }

        Toggle {
            checked: root.live
            Layout.alignment: Qt.AlignVCenter
            onToggled: backend.toggleMode()
        }
    }
}
