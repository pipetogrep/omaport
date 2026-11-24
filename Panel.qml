import QtQuick
import QtQuick.Controls
import QtQuick.Layouts
import Quickshell
import Quickshell.Io
import qs.Commons
import qs.Commons as Commons
import qs.Ui

Panel {
  id: root
  moduleName: "pipetogrep.omaport"
  ipcTarget: "pipetogrep.omaport"
  implicitWidth: button.implicitWidth
  implicitHeight: button.implicitHeight

  Commons.Translations {
    id: strings
    directory: Qt.resolvedUrl("i18n")
  }

  readonly property color foreground: bar ? bar.foreground : Commons.Color.foreground
  readonly property color accent: Commons.Color.accent
  readonly property color urgent: bar ? bar.urgent : Commons.Color.urgent
  readonly property color dim: Qt.darker(foreground, 1.55)
  readonly property string fontFamily: bar ? bar.fontFamily : Style.font.family
  readonly property color hoverFill: bar ? Style.hoverFillFor(bar.foreground, Commons.Color.accent) : "transparent"
  readonly property color selectedFill: bar ? Style.selectedFillFor(bar.foreground, Commons.Color.accent) : "transparent"

  readonly property bool showCount: Boolean(setting("showCount", true))
  property bool showAllPorts: Boolean(setting("showAllPorts", false))
  readonly property int refreshIntervalSec: Math.max(3, Number(setting("refreshIntervalSec", 10)) || 10)

  property string query: ""
  property int selectedIndex: 0
  property bool cursorActive: false

  readonly property var filteredServers: {
    var list = service.servers || []
    var q = query.trim().toLowerCase()
    if (q === "") return list
    var out = []
    for (var i = 0; i < list.length; i++) {
      var s = list[i]
      var hay = (
        ":" + String(s.port || "") + " " +
        String(s.project || "") + " " +
        String(s.name || "") + " " +
        String(s.cwdShort || "") + " " +
        String(s.branch || "") + " " +
        String(s.pid || "")
      ).toLowerCase()
      if (hay.indexOf(q) !== -1) out.push(s)
    }
    return out
  }

  function clampSelection() {
    if (filteredServers.length === 0) {
      selectedIndex = 0
      return
    }
    if (selectedIndex < 0) selectedIndex = 0
    if (selectedIndex >= filteredServers.length) selectedIndex = filteredServers.length - 1
  }

  function selectedServer() {
    if (filteredServers.length === 0) return null
    var idx = Math.max(0, Math.min(selectedIndex, filteredServers.length - 1))
    return filteredServers[idx]
  }

  function moveSelection(delta) {
    if (filteredServers.length === 0) return
    cursorActive = true
    selectedIndex = Math.max(0, Math.min(filteredServers.length - 1, selectedIndex + delta))
    var listView = panel.loadedContent ? panel.loadedContent.serverList : null
    if (listView) listView.positionViewAtIndex(selectedIndex, ListView.Contain)
  }

  function openSelected() {
    var srv = selectedServer()
    if (!srv) return
    service.openServer(srv)
    root.close()
  }

  function killSelected() {
    var srv = selectedServer()
    if (!srv) return
    service.killServer(srv)
  }

  onFilteredServersChanged: clampSelection()
  onOpenedChanged: {
    if (opened) {
      cursorActive = false
      selectedIndex = 0
      service.refresh()
    } else {
      query = ""
    }
  }

  Service {
    id: service
    panelOpen: root.opened
    showAllPorts: root.showAllPorts
    refreshIntervalSec: root.refreshIntervalSec
  }

  WidgetButton {
    id: button
    anchors.fill: parent
    bar: root.bar
    labelVisible: false
    hasVisualContent: true
    active: root.opened
    readonly property bool hasCount: root.showCount && !vertical && service.servers.length > 0
    readonly property string countText: hasCount ? Commons.I18n.formatNumber(service.servers.length) : ""
    readonly property color iconTint: active && useActiveColor
      ? activeColor
      : (service.servers.length > 0 ? root.barForeground : Qt.darker(root.barForeground, 1.55))
    fixedWidth: vertical
      ? -1
      : (hasCount
        ? Math.round(Style.bar.iconSlot + barCountLabel.implicitWidth + Style.space(6))
        : Style.bar.iconSlot)
    fixedHeight: vertical ? Style.bar.iconSlot : -1
    tooltipText: service.servers.length > 0
      ? strings.tn("tooltipActive", service.servers.length).arg(Commons.I18n.formatNumber(service.servers.length))
      : strings.t("tooltipEmpty")
    onPressed: function(buttonCode) {
      if (buttonCode === Qt.MiddleButton) service.refresh()
      else root.toggle()
    }

    Row {
      anchors.centerIn: parent
      spacing: Style.space(5)
      scale: button.pressScale

      Item {
        width: Style.bar.iconCanvas
        height: Style.bar.iconCanvas
        anchors.verticalCenter: parent.verticalCenter

        Rectangle {
          readonly property real dotSize: Math.max(8, Math.round(Style.bar.iconSize * 0.65))
          width: dotSize
          height: dotSize
          radius: dotSize / 2
          anchors.centerIn: parent
          antialiasing: true
          color: service.servers.length > 0 ? button.iconTint : "transparent"
          border.width: service.servers.length > 0 ? 0 : Math.max(1.5, Style.spaceReal(1.5))
          border.color: button.iconTint
        }
      }

      PlainText {
        id: barCountLabel
        visible: button.hasCount
        anchors.verticalCenter: parent.verticalCenter
        text: button.countText
        font.family: button.fontFamily
        font.pixelSize: Style.font.body
        color: button.iconTint
      }
    }
  }

  KeyboardPanel {
    id: panel
    anchorItem: button
    owner: root
    bar: root.bar
    open: root.opened
    focusTarget: panel.loadedContent
    contentWidth: panel.fittedContentWidth(Style.space(420))
    contentHeight: panel.fittedContentHeight(panel.loadedContentHeight, Style.space(520))

    content: PanelKeyCatcher {
      id: keyCatcher
      property alias serverList: serverList
      implicitHeight: contentColumn.implicitHeight
      anchors.fill: parent

      onMoveRequested: function(dx, dy) {
        if (dy !== 0) root.moveSelection(dy)
      }
      onActivateRequested: root.openSelected()
      onCloseRequested: root.close()
      onTabRequested: function(direction) { root.switchPanel(direction) }
      onTextKey: function(t) {
        if (t === "r" || t === "R") service.refresh()
        else if (t === "a" || t === "A") root.showAllPorts = !root.showAllPorts
        else if (t === "o" || t === "O") root.openSelected()
        else if (t === "x" || t === "X") root.killSelected()
        else if (t === "/" && searchBox.visible) searchInput.forceActiveFocus()
      }

      ColumnLayout {
        id: contentColumn
        width: parent.width
        spacing: Style.space(10)

        // Header bar: title + count + Dev/All segmented toggle + refresh
        RowLayout {
          Layout.fillWidth: true
          Layout.leftMargin: Style.space(10)
          Layout.rightMargin: Style.space(2)
          spacing: Style.space(8)

          PlainText {
            Layout.alignment: Qt.AlignVCenter
            text: root.showAllPorts ? strings.t("titleAll") : strings.t("title")
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.title
            font.weight: Font.DemiBold
          }

          Rectangle {
            visible: service.servers.length > 0
            Layout.alignment: Qt.AlignVCenter
            color: Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.18)
            radius: Math.min(4, Style.cornerRadius)
            implicitWidth: countText.implicitWidth + Style.space(10)
            implicitHeight: countText.implicitHeight + Style.space(2)

            PlainText {
              id: countText
              anchors.centerIn: parent
              text: Commons.I18n.formatNumber(service.servers.length)
              color: root.accent
              font.family: root.fontFamily
              font.pixelSize: Style.font.caption
              font.weight: Font.DemiBold
            }
          }

          Item { Layout.fillWidth: true }

          // Clear Dev / All segmented toggle
          Rectangle {
            Layout.alignment: Qt.AlignVCenter
            color: Qt.rgba(root.foreground.r, root.foreground.g, root.foreground.b, 0.08)
            radius: Math.min(4, Style.cornerRadius)
            implicitWidth: modeRow.implicitWidth + Style.space(4)
            implicitHeight: modeRow.implicitHeight + Style.space(4)

            Row {
              id: modeRow
              anchors.centerIn: parent
              spacing: Style.space(2)

              Rectangle {
                radius: Math.min(3, Style.cornerRadius)
                color: !root.showAllPorts
                  ? Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.22)
                  : "transparent"
                implicitWidth: devModeLabel.implicitWidth + Style.space(12)
                implicitHeight: devModeLabel.implicitHeight + Style.space(6)

                PlainText {
                  id: devModeLabel
                  anchors.centerIn: parent
                  text: strings.t("modeDev")
                  color: !root.showAllPorts ? root.accent : root.dim
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                  font.weight: !root.showAllPorts ? Font.DemiBold : Font.Normal
                }

                MouseArea {
                  anchors.fill: parent
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.showAllPorts = false
                }
              }

              Rectangle {
                radius: Math.min(3, Style.cornerRadius)
                color: root.showAllPorts
                  ? Qt.rgba(root.accent.r, root.accent.g, root.accent.b, 0.22)
                  : "transparent"
                implicitWidth: allModeLabel.implicitWidth + Style.space(12)
                implicitHeight: allModeLabel.implicitHeight + Style.space(6)

                PlainText {
                  id: allModeLabel
                  anchors.centerIn: parent
                  text: strings.t("modeAll")
                  color: root.showAllPorts ? root.accent : root.dim
                  font.family: root.fontFamily
                  font.pixelSize: Style.font.caption
                  font.weight: root.showAllPorts ? Font.DemiBold : Font.Normal
                }

                MouseArea {
                  anchors.fill: parent
                  cursorShape: Qt.PointingHandCursor
                  onClicked: root.showAllPorts = true
                }
              }
            }
          }

          PanelActionButton {
            iconText: "\udb81\udc50"
            tooltipText: strings.t("refreshTooltip")
            foreground: root.dim
            hoverColor: root.foreground
            fontFamily: root.fontFamily
            enabled: !service.loading
            onClicked: service.refresh()
          }
        }

        // Filter box when there are many servers running (or a query is active)
        Rectangle {
          id: searchBox
          visible: service.servers.length > 5 || root.query !== ""
          Layout.fillWidth: true
          implicitHeight: Style.space(32)
          radius: Style.cornerRadius
          color: Qt.rgba(root.foreground.r, root.foreground.g, root.foreground.b, 0.06)
          border.width: searchInput.activeFocus ? 1 : 0
          border.color: root.accent

          RowLayout {
            anchors.fill: parent
            anchors.leftMargin: Style.space(10)
            anchors.rightMargin: Style.space(8)
            spacing: Style.space(6)

            PlainText {
              text: "\udb80\udf49"
              color: root.dim
              font.family: root.fontFamily
              font.pixelSize: Style.font.bodySmall
            }

            Item {
              Layout.fillWidth: true
              Layout.fillHeight: true

              PlainText {
                anchors.verticalCenter: parent.verticalCenter
                visible: searchInput.text === ""
                text: strings.t("searchPlaceholder")
                color: root.dim
                font.family: root.fontFamily
                font.pixelSize: Style.font.bodySmall
              }

              TextInput {
                id: searchInput
                anchors.fill: parent
                verticalAlignment: TextInput.AlignVCenter
                color: root.foreground
                font.family: root.fontFamily
                font.pixelSize: Style.font.bodySmall
                selectByMouse: true
                clip: true
                text: root.query
                onTextEdited: root.query = text
                Keys.onEscapePressed: {
                  if (root.query !== "") {
                    root.query = ""
                    keyCatcher.forceActiveFocus()
                  } else {
                    root.close()
                  }
                }
                Keys.onDownPressed: {
                  root.moveSelection(1)
                  keyCatcher.forceActiveFocus()
                }
                Keys.onUpPressed: {
                  root.moveSelection(-1)
                  keyCatcher.forceActiveFocus()
                }
                Keys.onReturnPressed: root.openSelected()
                Keys.onEnterPressed: root.openSelected()
              }
            }

            PanelActionButton {
              visible: root.query !== ""
              iconText: "\udb80\udd56"
              foreground: root.dim
              hoverColor: root.foreground
              fontFamily: root.fontFamily
              size: Style.space(20)
              onClicked: {
                root.query = ""
                keyCatcher.forceActiveFocus()
              }
            }
          }
        }

        // Error banner if binary or kill fails
        Rectangle {
          visible: service.lastError !== ""
          Layout.fillWidth: true
          implicitHeight: errorLabel.implicitHeight + Style.space(12)
          radius: Style.cornerRadius
          color: Qt.rgba(root.urgent.r, root.urgent.g, root.urgent.b, 0.14)

          PlainText {
            id: errorLabel
            anchors.fill: parent
            anchors.margins: Style.space(6)
            text: service.lastError
            color: root.urgent
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
            wrapMode: Text.WordWrap
          }
        }

        PanelSeparator {
          Layout.fillWidth: true
        }

        // Scrollable server list
        ListView {
          id: serverList
          visible: root.filteredServers.length > 0
          Layout.fillWidth: true
          readonly property real maxListHeight: Style.space(380)
          readonly property real naturalHeight: Math.max(Style.space(56), contentHeight)
          implicitHeight: Math.min(maxListHeight, naturalHeight)
          Layout.preferredHeight: implicitHeight
          clip: true
          spacing: Style.space(4)
          boundsBehavior: Flickable.StopAtBounds
          flickableDirection: Flickable.VerticalFlick
          interactive: contentHeight > height
          model: root.filteredServers

          ScrollBar.vertical: ScrollBar {
            policy: serverList.contentHeight > serverList.height
              ? ScrollBar.AsNeeded
              : ScrollBar.AlwaysOff
          }

          delegate: ServerRow {
            required property var modelData
            required property int index

            width: serverList.width - (serverList.contentHeight > serverList.height ? Style.space(8) : 0)
            server: modelData
            selected: root.cursorActive && root.selectedIndex === index
            killing: service.isKilling(modelData.pid)
            foreground: root.foreground
            dim: root.dim
            accent: root.accent
            urgent: root.urgent
            hoverFill: root.hoverFill
            selectedFill: root.selectedFill
            fontFamily: root.fontFamily
            openTooltip: strings.t("openTooltip")
            killTooltip: strings.t("killTooltip")
            pidText: String(modelData.pid)

            onHovered: {
              root.cursorActive = true
              root.selectedIndex = index
            }
            onOpenRequested: {
              service.openServer(modelData)
              root.close()
            }
            onKillRequested: service.killServer(modelData)
          }
        }

        // Empty state
        ColumnLayout {
          visible: root.filteredServers.length === 0
          Layout.fillWidth: true
          Layout.topMargin: Style.space(18)
          Layout.bottomMargin: Style.space(18)
          spacing: Style.space(6)

          Rectangle {
            readonly property real dotSize: Style.space(22)
            Layout.alignment: Qt.AlignHCenter
            Layout.preferredWidth: dotSize
            Layout.preferredHeight: dotSize
            radius: dotSize / 2
            antialiasing: true
            color: "transparent"
            border.width: Math.max(2, Style.spaceReal(2))
            border.color: root.dim
          }

          PlainText {
            Layout.alignment: Qt.AlignHCenter
            text: root.query !== "" ? strings.t("noMatches") : strings.t("emptyTitle")
            color: root.foreground
            font.family: root.fontFamily
            font.pixelSize: Style.font.body
            font.weight: Font.Medium
          }

          PlainText {
            visible: root.query === ""
            Layout.alignment: Qt.AlignHCenter
            text: strings.t("emptyHint")
            color: root.dim
            font.family: root.fontFamily
            font.pixelSize: Style.font.caption
          }
        }
      }
    }
  }
}
