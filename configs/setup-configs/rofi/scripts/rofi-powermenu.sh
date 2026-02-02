#!/bin/bash

options="  Shutdown\n  Reboot\n  Suspend\n  Lock\n  Logout"

# // --  Show menu -- //
# Note: the space in the prompt "capture screen" is necessary for it to look centered, change at ur own risk
chosen=$(echo -e "$options" | rofi -dmenu -i -p "    Power Menu " \
    -theme-str '
        window {
            width: 260px;
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
    "  Shutdown")
        systemctl poweroff
        ;;
    "  Reboot")
        systemctl reboot
        ;;
    "  Suspend")
        systemctl suspend
        ;;
    "  Lock")
        swaylock
        ;;
    "  Logout")
        niri msg action quit
        ;;
esac
