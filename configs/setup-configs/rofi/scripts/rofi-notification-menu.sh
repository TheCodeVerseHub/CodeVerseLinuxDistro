#!/bin/bash

options="󰂛  Toggle DND\n󰩺  Clear All\n󰒓  Open Settings"

# // --  Show menu -- //
# Note: the space in the prompt "capture screen" is necessary for it to look centered, change at ur own risk
chosen=$(echo -e "$options" | rofi -dmenu -i -p "   Notification Menu" \
    -theme-str '
        window {
            width: 300px;
            x-offset: 0;
            y-offset: 0;
        }
        inputbar {
            children: [prompt];
        }
        textbox-prompt-colon {
            enabled: false;
        }
        entry {
            enabled: false;
        }
    '
)

case $chosen in
    "󰂛  Toggle DND")
        swaync-client -d -sw
        ;;
    "󰩺  Clear All")
        swaync-client -C
        ;;
    "󰒓  Open Settings")
        swaync-client -t -sw
        ;;
esac
