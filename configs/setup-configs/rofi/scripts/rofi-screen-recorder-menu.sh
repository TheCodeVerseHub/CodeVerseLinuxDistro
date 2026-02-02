#!/usr/bin/env bash

# // -- variables -- //
OUTPUT_DIR="$HOME/Videos/Screen-recordings"
PAUSE_FILE="/tmp/wf-recorder-paused"

# // -- Create output directory if it doesn't exist -- //
mkdir -p "$OUTPUT_DIR"

# // -- Check recording status -- //
if pgrep -x "wf-recorder" > /dev/null; then
    if [ -f "$PAUSE_FILE" ]; then
        OPTIONS=" Resume Recording\n Stop Recording"
    else
        OPTIONS=" Pause Recording\n Stop Recording"
    fi
else
    OPTIONS=" Start Full Screen\n Start Area Selection\n Start Window"
fi

# // -- Show menu -- //
# Note: the space in the prompt "capture screen" is necessary for it to look centered, change at ur own risk
SELECTION=$(echo -e "$OPTIONS" | rofi -dmenu -i -p "        Record Screen" \
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

# // -- Do operations based on what user selected -- //
case "$SELECTION" in
    " Start Full Screen")
        FILENAME="$OUTPUT_DIR/recording_$(date +%Y%m%d_%H%M%S).mp4"
        wf-recorder -f "$FILENAME" &
        sleep 0.5
        [ -f "$PAUSE_FILE" ] && rm "$PAUSE_FILE"
        notify-send "Recording started" "Full screen"
        ;;
    " Start Area Selection")
        FILENAME="$OUTPUT_DIR/recording_$(date +%Y%m%d_%H%M%S).mp4"
        wf-recorder -g "$(slurp)" -f "$FILENAME" &
        sleep 0.5
        [ -f "$PAUSE_FILE" ] && rm "$PAUSE_FILE"
        notify-send "Recording started" "Selected area"
        ;;
    " Start Window")
        FILENAME="$OUTPUT_DIR/recording_$(date +%Y%m%d_%H%M%S).mp4"
        # - Get window geometry -
        GEOM=$(niri msg --json focused-window | jq -r '"\(.x),\(.y) \(.width)x\(.height)"')
        wf-recorder -g "$GEOM" -f "$FILENAME" &
        sleep 0.5
        [ -f "$PAUSE_FILE" ] && rm "$PAUSE_FILE"
        notify-send "Recording started" "Current window"
        ;;
    " Pause Recording")
        pkill -STOP wf-recorder
        touch "$PAUSE_FILE"
        notify-send "Recording paused" "Resume from menu"
        ;;
    " Resume Recording")
        pkill -CONT wf-recorder
        rm -f "$PAUSE_FILE"
        notify-send "Recording resumed"
        ;;
    " Stop Recording")
        # - Try graceful stop first, then force if needed -
        pkill -TERM wf-recorder
        rm -f "$PAUSE_FILE"
        # - Wait for wf-recorder to exit (max 3 seconds) -
        for i in {1..6}; do
            pgrep -x "wf-recorder" > /dev/null || break
            sleep 0.5
        done
        # - Force kill if still running -
        pkill -9 wf-recorder 2>/dev/null
        notify-send "Recording stopped" "Saved to $OUTPUT_DIR/"
        ;;
esac
