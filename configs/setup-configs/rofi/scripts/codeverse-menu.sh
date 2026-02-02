#!/usr/bin/env bash

SCRIPTS_DIR="$HOME/.config/rofi/scripts"

# // --  If no argument is provided, list the options -- //
if [ -z "$@" ]; then
    echo -en "\0prompt\x1fCVH Linux\n"
    echo -en "󰣆  Applications\n"
    echo -en "󰖯  Windows\n"
    echo -en "󰸉  Wallpaper Selector\n"
    echo -en "󰌌  Keybinds\n"
    echo -en "󰅌  Clipboard\n"
    echo -en "󰞅  Emoji Selector\n"
    echo -en "󰃬  Calculator\n"
    echo -en "󰖩  WiFi Manager\n"
    echo -en "⚙  Configs\n"
    echo -en "󰕾  Volume Controller\n"
    echo -en "󰃠  Brightness Controller\n"
    echo -en "⏻  Power Menu\n"
    echo -en "󰄀  Take Screenshot\n"
    echo -en "󰕧  Screen Recorder\n"
    echo -en "󰣀  SSH/Run\n"
    echo -en "󰂚  Notification Options\n"
    echo -n  "󰁹  Battery Options"
else
    # // -- Handle the selection -- //
    case "$@" in
        "󰣆  Applications")
            coproc ( rofi -show drun > /dev/null 2>&1 )
            ;;
        "󰖯  Windows")
            coproc ( rofi -show window > /dev/null 2>&1 )
            ;;
        "󰸉  Wallpaper Selector")
            coproc ( "$SCRIPTS_DIR/rofi-wallpaper-selector.sh" > /dev/null 2>&1 )
            ;;
        "󰌌  Keybinds")
            coproc ( "$SCRIPTS_DIR/rofi-keybind-menu.sh" > /dev/null 2>&1 )
            ;;
        "󰅌  Clipboard")
            coproc ( "$SCRIPTS_DIR/rofi-clipboard.sh" > /dev/null 2>&1 )
            ;;
        "󰞅  Emoji Selector")
            coproc ( "$SCRIPTS_DIR/rofi-emoji-selector.sh" > /dev/null 2>&1 )
            ;;
        "󰃬  Calculator")
            coproc ( "$SCRIPTS_DIR/rofi-calculator.sh" > /dev/null 2>&1 )
            ;;
        "󰖩  WiFi Manager")
            coproc ( "$SCRIPTS_DIR/rofi-wifi-menu.sh" > /dev/null 2>&1 )
            ;;
        "⚙  Configs")
            coproc ( "$SCRIPTS_DIR/rofi-configs-menu.sh" > /dev/null 2>&1 )
            ;;
        "󰕾  Volume Controller")
            coproc ( "$SCRIPTS_DIR/rofi-volume-selector.sh" > /dev/null 2>&1 )
            ;;
        "󰃠  Brightness Controller")
            coproc ( "$SCRIPTS_DIR/rofi-brightness-selector.sh" > /dev/null 2>&1 )
            ;;
        "⏻  Power Menu")
            coproc ( "$SCRIPTS_DIR/rofi-powermenu.sh" > /dev/null 2>&1 )
            ;;
        "󰄀  Take Screenshot")
            coproc ( "$SCRIPTS_DIR/rofi-screenshot-menu.sh" > /dev/null 2>&1 )
            ;;
        "󰕧  Screen Recorder")
            coproc ( "$SCRIPTS_DIR/rofi-screen-recorder-menu.sh" > /dev/null 2>&1 )
            ;;
        "󰣀  SSH/Run")
            coproc ( rofi -show run > /dev/null 2>&1 )
            ;;
        "󰂚  Notification Options")
            coproc ( "$SCRIPTS_DIR/rofi-notification-menu.sh" > /dev/null 2>&1 )
            ;;
        "󰁹  Battery Options")
            coproc ( "$SCRIPTS_DIR/rofi-battery-power-menu.sh" > /dev/null 2>&1 )
            ;;
    esac
fi

# Made by community manager MHIA(MHashir09)
