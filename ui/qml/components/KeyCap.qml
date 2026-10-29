// VietFlow Qt UI - Keyboard key cap
import QtQuick

Rectangle {
    id: root

    property string label: ""

    implicitWidth: Math.max(34, labelText.implicitWidth + 14)
    implicitHeight: 24

    radius: 5
    color: theme.surfaceAlt
    border.width: 1
    border.color: theme.border

    Text {
        id: labelText
        anchors.centerIn: parent
        text: root.label
        color: theme.textDim
        font.pixelSize: 11
        font.weight: Font.DemiBold
    }
}
