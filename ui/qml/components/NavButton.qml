// VietFlow Qt UI - Navigation item
import QtQuick
import QtQuick.Layouts

Item {
    id: root

    property string label
    property url icon
    property url iconActive
    property bool active: false

    signal clicked()

    implicitHeight: 40

    Rectangle {
        anchors.fill: parent
        radius: 8
        color: root.active ? theme.accent : (mouse.containsMouse ? theme.surfaceHover : "transparent")
        Behavior on color { ColorAnimation { duration: 120 } }
    }

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 11
        anchors.rightMargin: 11
        spacing: 10

        Image {
            source: root.active ? root.iconActive : root.icon
            sourceSize.width: 18
            sourceSize.height: 18
            Layout.alignment: Qt.AlignVCenter
            asynchronous: true
        }

        Text {
            Layout.fillWidth: true
            text: root.label
            color: root.active ? "#ffffff" : theme.textDim
            font.pixelSize: 13
            font.weight: root.active ? Font.DemiBold : Font.Normal
            elide: Text.ElideRight
            verticalAlignment: Text.AlignVCenter
        }
    }

    MouseArea {
        id: mouse
        anchors.fill: parent
        hoverEnabled: true
        cursorShape: Qt.PointingHandCursor
        onClicked: root.clicked()
    }
}
