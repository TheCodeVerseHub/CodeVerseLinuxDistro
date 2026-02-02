#!/bin/bash

SCREENSHOT_DIR="$HOME/Pictures/Screenshots"
mkdir -p "$SCREENSHOT_DIR"
FILENAME="$SCREENSHOT_DIR/screenshot_$(date +%Y%m%d_%H%M%S).png"

grim -g "$(slurp)" "$FILENAME" 2>/dev/null

if [ -f "$FILENAME" ]; then
    notify-send "Screenshot saved" "$FILENAME"
    wl-copy < "$FILENAME"
fi
