// VietFlow Qt UI - Toggle switch
import QtQuick

Item {
    id: root

    property bool checked: false
    signal toggled()

    implicitWidth: 46
    implicitHeight: 26

    Rectangle {
        anchors.fill: parent
        radius: 13
        color: root.checked ? theme.accent : "#39414d"
        Behavior on color { ColorAnimation { duration: 140 } }
    }

    Rectangle {
        id: knob
        width: 20
        height: 20
        radius: 10
        y: 3
        x: root.checked ? root.width - width - 3 : 3
        color: root.checked ? "#ffffff" : "#c3cbd6"
        Behavior on x { NumberAnimation { duration: 140; easing.type: Easing.InOutQuad } }
        Behavior on color { ColorAnimation { duration: 140 } }
    }

    MouseArea {
        anchors.fill: parent
        cursorShape: Qt.PointingHandCursor
        onClicked: root.toggled()
    }

    Accessible.role: Accessible.CheckBox
    Accessible.name: "Toggle"
    Accessible.checked: root.checked
}
