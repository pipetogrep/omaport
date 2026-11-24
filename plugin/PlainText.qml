import QtQuick

// Safe Text wrapper for plugin UI: forces PlainText so dynamic process names,
// directory paths, and git branches can never be parsed as rich text/markup.
Text {
  textFormat: Text.PlainText
  renderType: Text.NativeRendering
}
