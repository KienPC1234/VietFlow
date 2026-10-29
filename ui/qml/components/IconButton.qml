// VietFlow Qt UI - Small icon/row action button
import QtQuick

Item {
    id: root

    property string glyph: "×"
    property color glyphColor: theme.textFaint
    property color glyphHoverColor: theme.danger
    signal clicked()

    implicitWidth: 26
    implicitHeight: 26

    Rectangle {
        anchors.fill: parent
        radius: 6
        color: mouse.containsMouse ? "#3a2229" : "transparent"
        Behavior on color { ColorAnimation { duration: 100 } }
    }

    Text {
        anchors.centerIn: parent
        text: root.glyph
        color: mouse.containsMouse ? root.glyphHoverColor : root.glyphColor
        font.pixelSize: 16
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
