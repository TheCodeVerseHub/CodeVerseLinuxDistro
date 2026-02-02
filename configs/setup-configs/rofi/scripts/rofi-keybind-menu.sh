#!/usr/bin/env bash

# // -- variables -- //
KEYBINDS_DIR="$HOME/.config/niri/Keybindings"
TEMP_FILE="/tmp/niri-keybinds.txt"

# // -- clear temp file -- //
> "$TEMP_FILE"

# //  -- Function to extract and format keybinds from a file -- //
parse_keybinds() {
    local file="$1"

    awk '
        /spawn/ && /\{/ {
            # Extract key combination (first field)
            key = $1

            # Extract spawn command
            if (match($0, /spawn "([^"]*)"/, cmd)) {
                print key ": " cmd[1]
            } else if (match($0, /spawn-sh "([^"]*)"/, cmd)) {
                # Clean up common commands
                action = cmd[1]
                gsub(/wpctl set-volume.*0\.1\+.*/, "Volume Up", action)
                gsub(/wpctl set-volume.*0\.1-/, "Volume Down", action)
                gsub(/wpctl set-mute.*SINK.*toggle/, "Mute Audio", action)
                gsub(/wpctl set-mute.*SOURCE.*toggle/, "Mute Mic", action)
                gsub(/playerctl play-pause/, "Play/Pause", action)
                gsub(/playerctl stop/, "Stop", action)
                gsub(/playerctl previous/, "Previous Track", action)
                gsub(/playerctl next/, "Next Track", action)
                print key ": " action
            }
        }
        /close-window/ {
            print $1 ": Close window"
        }
        /quit/ && !/Audio/ {
            print $1 ": Quit niri"
        }
        /fullscreen/ {
            print $1 ": Toggle fullscreen"
        }
        /focus-(left|right|up|down)/ {
            direction = $0
            gsub(/.*focus-/, "", direction)
            gsub(/;.*/, "", direction)
            print $1 ": Focus " direction
        }
        /move-(left|right|up|down)/ {
            direction = $0
            gsub(/.*move-/, "", direction)
            gsub(/;.*/, "", direction)
            print $1 ": Move " direction
        }
    ' "$file" >> "$TEMP_FILE"
}

# // -- Parse all keybind files -- //
for file in "$KEYBINDS_DIR"/*.kdl; do
    if [ -f "$file" ]; then
        parse_keybinds "$file"
    fi
done

# // --  rofi menu -- //
cat "$TEMP_FILE" | rofi -dmenu -i \
    -theme-str '
        window { width: 900px; }
    '

# // -- removing the temporary file as its not needed anymore -- //
rm -f "$TEMP_FILE"
