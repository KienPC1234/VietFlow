// VietFlow Qt UI - Themed drop-down selector
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Item {
    id: root

    property var items: []
    property int currentIndex: 0
    signal activated(int index)

    implicitWidth: 200
    implicitHeight: 38
    Layout.minimumWidth: 160

    Rectangle {
        id: box
        anchors.fill: parent
        radius: 6
        color: mouse.containsMouse || popup.visible ? theme.surfaceAlt : theme.surface
        border.width: 1
        border.color: popup.visible ? theme.accent : theme.border
        Behavior on color { ColorAnimation { duration: 100 } }
        Behavior on border.color { ColorAnimation { duration: 100 } }

        Text {
            anchors.left: parent.left
            anchors.leftMargin: 12
            anchors.right: chevron.left
            anchors.rightMargin: 8
            anchors.verticalCenter: parent.verticalCenter
            text: root.items.length > 0
                  ? String(root.items[Math.max(0, Math.min(root.currentIndex, root.items.length - 1))])
                  : ""
            color: theme.text
            font.pixelSize: 12
            elide: Text.ElideRight
        }

        Text {
            id: chevron
            anchors.right: parent.right
            anchors.rightMargin: 11
            anchors.verticalCenter: parent.verticalCenter
            text: "⌄"
            color: theme.textDim
            font.pixelSize: 14
        }

        MouseArea {
            id: mouse
            anchors.fill: parent
            hoverEnabled: true
            cursorShape: Qt.PointingHandCursor
            onClicked: popup.visible ? popup.close() : popup.open()
        }
    }

    Popup {
        id: popup
        x: 0
        y: root.height + 4
        width: root.width
        padding: 4
        closePolicy: Popup.CloseOnEscape | Popup.CloseOnPressOutside

        background: Rectangle {
            color: theme.surfaceAlt
            radius: 8
            border.width: 1
            border.color: theme.border
        }

        contentItem: ListView {
            implicitHeight: Math.min(6, root.items.length) * 34
            clip: true
            model: root.items
            boundsBehavior: Flickable.StopAtBounds

            delegate: Item {
                width: popup.availableWidth
                height: 34

                Rectangle {
                    anchors.fill: parent
                    radius: 5
                    color: rowMouse.containsMouse
                           ? theme.surfaceHover
                           : (index === root.currentIndex ? theme.accentSoft : "transparent")
                }

                Text {
                    anchors.left: parent.left
                    anchors.leftMargin: 10
                    anchors.right: parent.right
                    anchors.rightMargin: 10
                    anchors.verticalCenter: parent.verticalCenter
                    text: modelData
                    color: theme.text
                    font.pixelSize: 12
                    elide: Text.ElideRight
                }

                MouseArea {
                    id: rowMouse
                    anchors.fill: parent
                    hoverEnabled: true
                    cursorShape: Qt.PointingHandCursor
                    onClicked: {
                        root.activated(index);
                        popup.close();
                    }
                }
            }
        }
    }
}
