#!/bin/bash

# // -- Get active power mode by parsing powerprofilesctl list -- //
current_mode=$(powerprofilesctl list | awk '/^\*/ {gsub(":", "", $2); print $2}')

# // -- Define modes and icons -- //
declare -A icons=(
    ["performance"]="󰻠"
    ["balanced"]="󱍈"
    ["power-saver"]="󰐥"
)

# // -- Build menu entries with checkmark for current mode -- //
menu_entries=""
for mode in performance balanced power-saver; do
    display="${icons[$mode]}  ${mode^}"
    [ "$mode" = "$current_mode" ] && display="$display ✓"
    menu_entries+="$display\n"
done

# // -- Show Rofi menu -- //
chosen=$(echo -e "$menu_entries" | rofi -dmenu -i -p "    Battery Options" \
    -theme-str '
        window { width: 300px; }
        inputbar { children: [prompt]; }
        textbox-prompt-colon { enabled: false; }
        entry { enabled: false; }
    '
)

#  // -- Apply selected mode -- //
[ -z "$chosen" ] && exit 0

case "$chosen" in
    *Performance*)
        powerprofilesctl set performance
        ;;
    *Balanced*)
        powerprofilesctl set balanced
        ;;
    *Power\ Saver*)
        powerprofilesctl set power-saver
        ;;
    *)
        exit 0
        ;;
esac

notify-send "Power Mode" "Switched to $chosen"
