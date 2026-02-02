#!/bin/bash

# // ask PipeWire/PulseAudio for the current volume of your default audio output //
current=$(pactl get-sink-volume @DEFAULT_SINK@ | grep -Po '\d+(?=%)' | head -1)

# // show menu with current highlighted //
new=$(echo -e "➯   0\n➯  10\n➯  20\n➯  30\n➯  40\n➯  50\n➯  60\n➯  70\n➯  80\n➯  90\n➯  100" \
      | rofi -dmenu \
      -selected-row $((current/10)) \
      -theme-str '
          textbox-prompt-colon { str: " Volume-level:"; }
          entry { placeholder: ""; }
      ')

# //  change volume to selected value //
if [ -n "$new" ]; then
    volume=$(echo "$new" | grep -o '[0-9]\+')
    pactl set-sink-volume @DEFAULT_SINK@ "${volume}%"
    swayosd-client --output-volume "${volume}"
fi
