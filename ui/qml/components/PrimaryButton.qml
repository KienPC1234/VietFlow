// VietFlow Qt UI - Primary action button
import QtQuick
import QtQuick.Controls

Button {
    id: root

    implicitHeight: 38
    leftPadding: 18
    rightPadding: 18
    font.pixelSize: 12
    font.weight: Font.DemiBold

    contentItem: Text {
        text: root.text
        color: "#ffffff"
        font: root.font
        horizontalAlignment: Text.AlignHCenter
        verticalAlignment: Text.AlignVCenter
        elide: Text.ElideRight
    }

    background: Rectangle {
        radius: 8
        color: root.down ? theme.accentStrong : (root.hovered ? theme.accentHover : theme.accent)
        Behavior on color { ColorAnimation { duration: 100 } }
    }
}
