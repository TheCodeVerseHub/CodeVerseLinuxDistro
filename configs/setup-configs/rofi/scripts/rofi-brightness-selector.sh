#!/bin/bash

# // Get current brightness percentage //
current=$(brightnessctl g)
max=$(brightnessctl m)
current=$(( 100 * current / max ))

# // Show menu with current highlighted //
new=$(echo -e "➯   0\n➯  10\n➯  20\n➯  30\n➯  40\n➯  50\n➯  60\n➯  70\n➯  80\n➯  90\n➯  100" \
      | rofi -dmenu \
      -selected-row $((current/10)) \
      -theme-str "
          textbox-prompt-colon { str: ' Brightness-level:'; }
          entry { placeholder: ''; }
      ")

# // Change brightness to selected value //
if [ -n "$new" ]; then
    brightness=$(echo "$new" | grep -o '[0-9]\+')
    brightnessctl s "${brightness}%"
    swayosd-client --brightness "${brightness}"
fi
