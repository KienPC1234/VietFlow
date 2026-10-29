// VietFlow Qt UI - Scrollable page container with consistent padding
import QtQuick
import QtQuick.Controls
import QtQuick.Layouts

Flickable {
    id: root

    default property alias contentData: column.data

    clip: true
    contentWidth: width
    contentHeight: column.implicitHeight + 40
    boundsBehavior: Flickable.StopAtBounds

    ScrollBar.vertical: ScrollBar {
        policy: ScrollBar.AsNeeded

        background: Rectangle {
            implicitWidth: 6
            color: "transparent"
        }

        contentItem: Rectangle {
            implicitWidth: 6
            implicitHeight: 40
            radius: 3
            color: theme.border
        }
    }

    ColumnLayout {
        id: column
        x: 20
        y: 20
        width: root.width - 40
        spacing: 10
    }
}
