#!/bin/bash

OUTPUT_DIR="$HOME/Videos/Screen-recordings"
mkdir -p "$OUTPUT_DIR"

if pgrep -x "wf-recorder" > /dev/null; then
    pkill -SIGINT wf-recorder
    notify-send "Recording stopped" "Video saved to $OUTPUT_DIR/"
else
    FILENAME="$OUTPUT_DIR/recording_$(date +%Y%m%d_%H%M%S).mp4"
    wf-recorder -a -f "$FILENAME" &
    sleep 0.5
    if pgrep -x "wf-recorder" > /dev/null; then
        notify-send "Recording started" "Press keybind again to stop"
    else
        notify-send "Recording failed" "wf-recorder could not start"
    fi
fi
