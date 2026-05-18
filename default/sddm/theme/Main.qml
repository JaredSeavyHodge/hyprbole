import QtQuick 2.0
import SddmComponents 2.0

Rectangle {
    id: container
    width: 1280
    height: 720
    color: "#111318"

    LayoutMirroring.enabled: Qt.locale().textDirection == Qt.RightToLeft
    LayoutMirroring.childrenInherit: true

    property int sessionIndex: {
        for (var i = 0; i < sessionModel.rowCount(); i++) {
            var name = (sessionModel.data(sessionModel.index(i, 0), Qt.DisplayRole) || "").toString()
            if (name.indexOf("uwsm") !== -1)
                return i
        }
        return sessionModel.lastIndex >= 0 ? sessionModel.lastIndex : 0
    }

    function login() {
        if (name.text == "") {
            return
        }

        errorMessage.color = config.Muted
        errorMessage.text = textConstants.prompt
        sddm.login(name.text, password.text, sessionIndex)
    }

    TextConstants { id: textConstants }

    Connections {
        target: sddm

        onLoginSucceeded: {
            errorMessage.color = config.Muted
            errorMessage.text = textConstants.loginSucceeded
        }

        onLoginFailed: {
            password.text = ""
            errorMessage.color = "#d97878"
            errorMessage.text = textConstants.loginFailed
        }
    }

    Image {
        id: background
        anchors.fill: parent
        source: config.backgroundBlur
        fillMode: Image.PreserveAspectCrop
        smooth: true
        asynchronous: false
        visible: status == Image.Ready
    }

    Rectangle {
        anchors.fill: parent
        color: "#111318"
        opacity: 0.44
    }

    Rectangle {
        width: parent.width * 0.42
        height: parent.height
        anchors.left: parent.left
        color: "#171b23"
        opacity: 0.62
    }

    Column {
        anchors.left: parent.left
        anchors.top: parent.top
        anchors.leftMargin: 72
        anchors.topMargin: 60
        spacing: 8

        Text {
            text: "hyprbole"
            color: config.Accent
            font.pixelSize: 34
            font.bold: true
            style: Text.Raised
            styleColor: "#ee05070b"
        }

        Rectangle {
            width: 220
            height: 1
            color: "#2c3442"
            opacity: 0.9
        }

    }

    Column {
        anchors.left: parent.left
        anchors.verticalCenter: parent.verticalCenter
        anchors.leftMargin: 72
        width: 420
        spacing: 12

        Text {
            text: textConstants.userName
            color: config.Muted
            font.pixelSize: 15
            font.bold: true
            style: Text.Raised
            styleColor: "#ee05070b"
        }

        Rectangle {
            width: parent.width
            height: 42
            radius: 10
            color: "#171b23"
            border.width: 1
            border.color: name.activeFocus ? "#626b7d" : "#2c3442"

            TextInput {
                id: name
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                verticalAlignment: Text.AlignVCenter
                color: config.Foreground
                selectionColor: "#394150"
                selectedTextColor: config.Foreground
                font.pixelSize: 17
                text: userModel.lastUser
                cursorDelegate: Rectangle {
                    width: 2
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    color: config.Foreground
                }
                KeyNavigation.tab: password
                Keys.onPressed: {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        login()
                        event.accepted = true
                    }
                }
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "Username"
                color: "#697284"
                font.pixelSize: 17
                style: Text.Raised
                styleColor: "#ee05070b"
                visible: name.text == "" && !name.activeFocus
            }
        }

        Text {
            text: textConstants.password
            color: config.Muted
            font.pixelSize: 15
            font.bold: true
            style: Text.Raised
            styleColor: "#ee05070b"
        }

        Rectangle {
            width: parent.width
            height: 42
            radius: 10
            color: "#171b23"
            border.width: 1
            border.color: password.activeFocus ? "#626b7d" : "#2c3442"

            TextInput {
                id: password
                anchors.fill: parent
                anchors.leftMargin: 14
                anchors.rightMargin: 14
                verticalAlignment: Text.AlignVCenter
                color: config.Foreground
                selectionColor: "#394150"
                selectedTextColor: config.Foreground
                font.pixelSize: 17
                echoMode: TextInput.Password
                cursorDelegate: Rectangle {
                    width: 2
                    anchors.top: parent.top
                    anchors.bottom: parent.bottom
                    color: config.Foreground
                }
                KeyNavigation.backtab: name
                Keys.onPressed: {
                    if (event.key === Qt.Key_Return || event.key === Qt.Key_Enter) {
                        login()
                        event.accepted = true
                    }
                }
            }

            Text {
                anchors.left: parent.left
                anchors.leftMargin: 14
                anchors.verticalCenter: parent.verticalCenter
                text: "Password"
                color: "#697284"
                font.pixelSize: 17
                style: Text.Raised
                styleColor: "#ee05070b"
                visible: password.text == "" && !password.activeFocus
            }
        }

        Text {
            id: errorMessage
            width: parent.width
            text: textConstants.prompt
            color: config.Muted
            wrapMode: Text.WordWrap
            font.pixelSize: 15
            style: Text.Raised
            styleColor: "#ee05070b"
        }

        Row {
            spacing: 10

            Rectangle {
                id: loginButton
                width: 130
                height: 40
                radius: 10
                color: "#242b36"
                border.width: 1
                border.color: "#394150"

                Text {
                    anchors.centerIn: parent
                    text: textConstants.login
                    color: config.Foreground
                    font.pixelSize: 15
                    font.bold: true
                    style: Text.Raised
                    styleColor: "#ee05070b"
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: login()
                }
            }

            Rectangle {
                width: 110
                height: 40
                radius: 10
                color: "#171b23"
                border.width: 1
                border.color: "#2c3442"

                Text {
                    anchors.centerIn: parent
                    text: textConstants.reboot
                    color: config.Foreground
                    font.pixelSize: 15
                    style: Text.Raised
                    styleColor: "#ee05070b"
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: sddm.reboot()
                }
            }

            Rectangle {
                width: 110
                height: 40
                radius: 10
                color: "#171b23"
                border.width: 1
                border.color: "#2c3442"

                Text {
                    anchors.centerIn: parent
                    text: textConstants.shutdown
                    color: config.Foreground
                    font.pixelSize: 15
                    style: Text.Raised
                    styleColor: "#ee05070b"
                }

                MouseArea {
                    anchors.fill: parent
                    onClicked: sddm.powerOff()
                }
            }
        }
    }

    Column {
        anchors.left: parent.left
        anchors.bottom: parent.bottom
        anchors.leftMargin: 72
        anchors.bottomMargin: 50
        spacing: 4

        Text {
            id: clockText
            color: config.Foreground
            font.pixelSize: 42
            font.bold: true
            style: Text.Raised
            styleColor: "#ee05070b"
        }

        Text {
            id: dateText
            color: config.Muted
            font.pixelSize: 18
            style: Text.Raised
            styleColor: "#ee05070b"
        }
    }

    Timer {
        interval: 1000
        repeat: true
        running: true
        triggeredOnStart: true
        onTriggered: {
            var now = new Date()
            clockText.text = Qt.formatDateTime(now, "HH:mm")
            dateText.text = Qt.formatDateTime(now, "dddd, MMMM d")
        }
    }

    Component.onCompleted: {
        if (name.text == "")
            name.focus = true
        else
            password.focus = true
    }
}
