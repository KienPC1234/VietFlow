// VietFlow Qt UI - TextArea with dark styling
import QtQuick
import QtQuick.Controls

TextArea {
    id: root

    color: theme.text
    placeholderTextColor: theme.textFaint
    font.pixelSize: 12
    selectionColor: theme.accent
    selectedTextColor: "#ffffff"
    wrapMode: TextArea.Wrap
    leftPadding: 12
    rightPadding: 12
    topPadding: 10
    bottomPadding: 10

    background: Rectangle {
        radius: 6
        color: theme.surface
        border.width: 1
        border.color: root.activeFocus ? theme.accent : theme.border
        Behavior on border.color { ColorAnimation { duration: 100 } }
    }
}
