// VietFlow Qt UI - Settings card
import QtQuick
import QtQuick.Layouts

Rectangle {
    id: root

    property string title
    property string description
    default property alias contentData: controls.data

    implicitHeight: 64
    Layout.fillWidth: true

    color: theme.surface
    radius: theme.radius
    border.width: 1
    border.color: theme.border

    RowLayout {
        anchors.fill: parent
        anchors.leftMargin: 18
        anchors.rightMargin: 18
        anchors.topMargin: 11
        anchors.bottomMargin: 11
        spacing: 18

        ColumnLayout {
            Layout.fillWidth: true
            spacing: 3

            Text {
                Layout.fillWidth: true
                text: root.title
                color: theme.text
                font.pixelSize: 13
                font.weight: Font.DemiBold
                elide: Text.ElideRight
            }

            Text {
                Layout.fillWidth: true
                visible: root.description.length > 0
                text: root.description
                color: theme.textDim
                font.pixelSize: 11
                wrapMode: Text.WordWrap
            }
        }

        RowLayout {
            id: controls
            spacing: 10
            Layout.alignment: Qt.AlignVCenter
        }
    }
}
