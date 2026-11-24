import QtQuick
import Quickshell
import Quickshell.Io

Item {
  id: root

  property bool panelOpen: false
  property bool showAllPorts: false
  property int refreshIntervalSec: 10

  property var servers: []
  property bool loading: false
  property string lastError: ""
  property var killingPids: ({})

  readonly property string binaryPath: {
    var resolved = String(Qt.resolvedUrl("omaport"))
    return resolved.indexOf("file://") === 0 ? resolved.slice(7) : resolved
  }

  function isKilling(pid) {
    return !!killingPids[String(pid)]
  }

  function setKilling(pid, active) {
    var next = Object.assign({}, killingPids)
    if (active) next[String(pid)] = true
    else delete next[String(pid)]
    killingPids = next
  }

  function refresh() {
    if (scanProcess.running) return
    loading = true
    var cmd = [binaryPath, "list", "--json"]
    if (showAllPorts) cmd.push("--all")
    scanProcess.command = cmd
    scanProcess.running = true
  }

  function openServer(server) {
    if (!server) return
    var port = Number(server.port)
    if (!isFinite(port) || port < 1 || port > 65535) return
    var url = "http://localhost:" + Math.floor(port)
    Quickshell.execDetached(["omarchy-launch-browser", url])
  }

  function killServer(server) {
    if (!server) return
    var pid = Number(server.pid)
    var port = Number(server.port)
    if (!isFinite(pid) || pid <= 1) return
    if (isKilling(pid)) return

    setKilling(pid, true)
    lastError = ""

    var args = [binaryPath, "kill", String(Math.floor(pid)), "--force"]
    if (isFinite(port) && port >= 1 && port <= 65535) {
      args.push("--port", String(Math.floor(port)))
    }

    var killer = killProcessComponent.createObject(root, {
      targetPid: Math.floor(pid),
      command: args
    })
    if (killer) {
      killer.running = true
    } else {
      setKilling(pid, false)
    }
  }

  onShowAllPortsChanged: refresh()
  onPanelOpenChanged: if (panelOpen) refresh()

  Component.onCompleted: refresh()

  Timer {
    id: pollTimer
    interval: root.panelOpen
      ? 4000
      : Math.max(3000, Math.min(300000, root.refreshIntervalSec * 1000))
    running: true
    repeat: true
    onTriggered: root.refresh()
  }

  Process {
    id: scanProcess
    stdout: StdioCollector {
      onStreamFinished: {
        try {
          var parsed = JSON.parse(this.text || "{}")
          root.servers = Array.isArray(parsed.servers) ? parsed.servers : []
          root.lastError = ""
        } catch (e) {
          root.lastError = "Failed to parse omaport output"
        }
      }
    }
    stderr: StdioCollector {
      onStreamFinished: {
        var msg = (this.text || "").trim()
        if (msg !== "") root.lastError = msg
      }
    }
    onExited: function(exitCode) {
      root.loading = false
      if (exitCode !== 0 && root.lastError === "") {
        root.lastError = "omaport exited with code " + exitCode
      }
    }
  }

  Component {
    id: killProcessComponent
    Process {
      id: kp
      property int targetPid: 0
      stderr: StdioCollector {
        onStreamFinished: {
          var msg = (this.text || "").trim()
          if (msg !== "") root.lastError = msg
        }
      }
      onExited: function(exitCode) {
        root.setKilling(kp.targetPid, false)
        if (exitCode !== 0 && root.lastError === "") {
          root.lastError = "Failed to kill PID " + kp.targetPid
        }
        root.refresh()
        kp.destroy()
      }
    }
  }
}
