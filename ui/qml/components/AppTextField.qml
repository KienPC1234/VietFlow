// VietFlow Qt UI - TextField with dark styling
import QtQuick
import QtQuick.Controls

TextField {
    id: root

    implicitHeight: 38
    leftPadding: 12
    rightPadding: 12
    color: theme.text
    placeholderTextColor: theme.textFaint
    font.pixelSize: 12
    selectionColor: theme.accent
    selectedTextColor: "#ffffff"

    background: Rectangle {
        radius: 6
        color: theme.surface
        border.width: 1
        border.color: root.activeFocus ? theme.accent : theme.border
        Behavior on border.color { ColorAnimation { duration: 100 } }
    }
}
