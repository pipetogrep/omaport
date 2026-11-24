import QtQuick
import QtQuick.Layouts
import qs.Commons
import qs.Commons as Commons
import qs.Ui

Item {
  id: root

  required property var server
  required property bool selected
  required property bool killing
  required property color foreground
  required property color dim
  required property color accent
  required property color urgent
  required property color hoverFill
  required property color selectedFill
  required property string fontFamily
  required property string openTooltip
  required property string killTooltip
  required property string pidText

  signal openRequested()
  signal killRequested()
  signal hovered()

  implicitHeight: Style.space(56)
  height: implicitHeight

  readonly property string titleText: {
    var proj = String(server.project || "")
    var name = String(server.name || "")
    return proj !== "" ? proj : (name !== "" ? name : "server")
  }

  readonly property string frameworkText: {
    var proj = String(server.project || "")
    var name = String(server.name || "")
    if (proj !== "" && name !== "" && proj.toLowerCase() !== name.toLowerCase()) {
      return name
    }
    return ""
  }

  readonly property string dirText: String(server.cwdShort || server.cwd || "")
  readonly property string branchText: String(server.branch || "")

  Rectangle {
    id: card
    anchors.fill: parent
    radius: Style.cornerRadius
    color: root.selected
      ? root.selectedFill
      : (rowMouse.containsMouse ? root.hoverFill : "transparent")

    Behavior on color { ColorAnimation { duration: Style.duration(80) } }

    MouseArea {
      id: rowMouse
      anchors.fill: parent
      hoverEnabled: true
      cursorShape: Qt.PointingHandCursor
      onEntered: root.hovered()
      onClicked: root.openRequested()
    }

    RowLayout {
      anchors.fill: parent
      anchors.leftMargin: Style.space(10)
      anchors.rightMargin: Style.space(8)
      anchors.topMargin: Style.space(7)
      anchors.bottomMargin: Style.space(7)
      spacing: Style.space(8)

      ColumnLayout {
        Layout.fillWidth: true
        Layout.alignment: Qt.AlignVCenter
        spacing: Style.space(3)

        // Top line: :port, project/name, optional framework pill, PID
        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(6)

          PlainText {
            text: ":" + String(root.server.port)
            color: root.accent
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            font.weight: Font.DemiBold
          }

          PlainText {
            Layout.fillWidth: true
            text: root.titleText
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            font.weight: Font.Medium
            elide: Text.ElideRight
          }

          PlainText {
            visible: root.frameworkText !== ""
            text: root.frameworkText
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          PlainText {
            visible: root.frameworkText !== ""
            text: "\u00b7"
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }

          PlainText {
            text: root.pidText
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }

        // Bottom line: working directory + git branch chip
        RowLayout {
          Layout.fillWidth: true
          spacing: Style.space(6)

          PlainText {
            Layout.fillWidth: true
            text: root.dirText !== "" ? root.dirText : String(root.server.command || "")
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.bodySmall
            elide: Text.ElideMiddle
          }

          Rectangle {
            visible: root.branchText !== ""
            color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.16)
            radius: Math.min(4, Style.cornerRadius)
            implicitWidth: Math.min(Style.space(135), branchRow.implicitWidth + Style.space(10))
            implicitHeight: branchRow.implicitHeight + Style.space(2)

            Row {
              id: branchRow
              anchors.centerIn: parent
              spacing: Style.space(4)

              PlainText {
                text: "\ue725"
                color: root.accent
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
                anchors.verticalCenter: parent.verticalCenter
              }

              PlainText {
                width: Math.min(implicitWidth, Style.space(110))
                text: root.branchText
                color: root.foreground
                font.family: root.fontFamily
                font.pixelSize: Style.font.caption
                elide: Text.ElideRight
                anchors.verticalCenter: parent.verticalCenter
              }
            }
          }
        }
      }

      // Action buttons: Open in browser + Kill
      Row {
        Layout.alignment: Qt.AlignVCenter
        spacing: Style.space(2)

        PanelActionButton {
          iconText: "\udb80\udfcc"
          tooltipText: root.openTooltip
          foreground: root.dim
          hoverColor: root.accent
          fontFamily: root.fontFamily
          enabled: !root.killing
          onClicked: root.openRequested()
        }

        PanelActionButton {
          iconText: root.killing ? "\udb81\udf51" : "\udb80\udd56"
          tooltipText: root.killTooltip
          foreground: root.dim
          hoverColor: root.urgent
          fontFamily: root.fontFamily
          enabled: !root.killing
          onClicked: root.killRequested()
        }
      }
    }
  }
}
