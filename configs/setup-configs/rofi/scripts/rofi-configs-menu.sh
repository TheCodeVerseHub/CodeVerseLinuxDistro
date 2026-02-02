#!/usr/bin/env bash

# // -- variables -- //
CONFIG_DIR="$HOME/.config"
EDITOR="nvim"

# // -- Select top-level config directory -- //
mapfile -t apps < <(find "$CONFIG_DIR" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' 2>/dev/null | sort -f)
(( ${#apps[@]} == 0 )) && { notify-send "Config Manager" "No config directories found"; exit 1; }

selected_app=$(printf '%s\n' "${apps[@]}" | rofi -dmenu -i -theme-str 'window { width: 720px; }')
[ -z "$selected_app" ] && exit 0
current_dir="$CONFIG_DIR/$selected_app"

# // -- Loop to navigate subdirectories -- //
while true; do
    # // - list files and subdirectories - //
    mapfile -t items < <(
        shopt -s nullglob
        for p in "$current_dir"/*; do
            [[ ! -e "$p" ]] && continue
            name="$(basename "$p")"
            case "$name" in .git|cache|Cache) continue ;; esac
            [[ -d "$p" ]] && echo "$name/" || echo "$name"
        done | sort -f
    )

    (( ${#items[@]} == 0 )) && {
        notify-send "Config Manager" "Nothing found in $(basename "$current_dir")"
        exit 1
    }

    # // -- Ask user to pick a file or directory -- //
    selected_item=$(printf '%s\n' "${items[@]}" | rofi -dmenu -i -theme-str 'window { width: 750px; }')
    [ -z "$selected_item" ] && exit 0

    # // --  If directory, descend; if file, open and exit -- //
    if [[ "$selected_item" == */ ]]; then
        current_dir="$current_dir/${selected_item%/}"
    else
        full_path="$current_dir/$selected_item"
        if [[ -f "$full_path" ]]; then
            kitty -e "$EDITOR" "$full_path" 2>/dev/null || notify-send "Config Manager" "Failed to open:\n$full_path"
        else
            notify-send "Config Manager" "File not found:\n$full_path"
        fi
        exit 0
    fi
done
