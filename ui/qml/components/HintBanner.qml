// VietFlow Qt UI - Inline hint banner
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    property string label
    property string message

    implicitHeight: 42
    Layout.fillWidth: true

    color: theme.accentSoft
    radius: 8
    border.width: 1
    border.color: "#2c4a78"

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 14
        anchors.rightMargin: 14
        spacing: 8

        Text {
            text: root.label
            color: "#7db2ff"
            font.pixelSize: 11
            font.weight: Font.DemiBold
            Layout.alignment: Qt.AlignVCenter
        }

        Text {
            Layout.fillWidth: true
            text: root.message
            color: "#cfe0ff"
            font.pixelSize: 11
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }
    }
}
