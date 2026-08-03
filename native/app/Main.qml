import QtQuick
import QtQuick.Controls
import OmniPackApp

ApplicationWindow {
    id: root
    width: 480
    height: 360
    visible: true
    title: "OmniPack (native smoke test)"

    PackerBridge { id: packer }

    Column {
        anchors.fill: parent
        anchors.margins: 16
        spacing: 12

        Button {
            text: "Pack sample"
            onClicked: resultText.text = packer.packSample()
        }

        Flickable {
            width: parent.width
            height: parent.height - 60
            contentHeight: resultText.height
            clip: true

            Text {
                id: resultText
                width: parent.width
                wrapMode: Text.WordWrap
                text: "Click \"Pack sample\" to run the ported C++ engine."
            }
        }
    }
}
