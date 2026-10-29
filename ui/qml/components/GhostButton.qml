// VietFlow Qt UI - Secondary / ghost button
import QtQuick
import QtQuick.Controls

Button {
    id: root

    implicitHeight: 38
    leftPadding: 16
    rightPadding: 16
    font.pixelSize: 12
    font.weight: Font.Medium

    contentItem: Text {
        text: root.text
        color: root.hovered ? theme.text : "#c8d0db"
        font: root.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: 8
        color: root.down ? "#20262f" : (root.hovered ? theme.surfaceHover : theme.surfaceAlt)
        border.width: 1
        border.color: root.hovered ? "#3a4350" : theme.border
        Behavior on color { ColorAnimation { duration: 100 } }
    }
}
