#!/usr/bin/env bash

# // -- variables -- //
SCREENSHOT_DIR="$HOME/Pictures/Screenshots"

# //  -- Create the directory if doesnt exist  --  //
mkdir -p "$SCREENSHOT_DIR"

# // -- Menu options -- //
OPTIONS=" Capture Full Screen\n Capture Area Selection\n Capture Active Window"

# // --  Show menu -- //
# Note: the space in the prompt "capture screen" is necessary for it to look centered, change at ur own risk
SELECTION=$(echo -e "$OPTIONS" | rofi -dmenu -i -p "        Capture Screen" \
    -theme-str '
        window {
            width: 380px;
            x-offset: 0;
            y-offset: 0;
        }
        inputbar {
            enabled: true;
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

# Exit if user pressed escape or made no selection
[ -z "$SELECTION" ] && exit 0

# // -- Name the screenshot -- //
FILENAME="$SCREENSHOT_DIR/screenshot_$(date +%Y%m%d_%H%M%S).png"

# // -- Do operations based on what user selected -- //
case "$SELECTION" in
    " Capture Full Screen")
        grim "$FILENAME"
        if [ -f "$FILENAME" ]; then
            notify-send "Screenshot saved" "$FILENAME"
            wl-copy < "$FILENAME"
        fi
        ;;
    " Capture Area Selection")
        grim -g "$(slurp)" "$FILENAME" 2>/dev/null
        if [ -f "$FILENAME" ]; then
            notify-send "Screenshot saved" "$FILENAME"
            wl-copy < "$FILENAME"
        fi
        ;;
    " Capture Active Window")
        GEOM=$(niri msg --json focused-window | jq -r '"\(.x),\(.y) \(.width)x\(.height)"')
        grim -g "$GEOM" "$FILENAME"
        if [ -f "$FILENAME" ]; then
            notify-send "Screenshot saved" "$FILENAME"
            wl-copy < "$FILENAME"
        fi
        ;;
esac
