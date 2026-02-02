#!/bin/bash

# // -- Variables -- //
WALLPAPER_DIR="$HOME/.config/niri/Wallpapers"
MPV_OPTIONS="no-audio loop panscan=1.0"
SET_SCRIPT="$HOME/.config/niri/scripts/set-wallpaper.sh"

# // -- get current active wallpaper -- //
current_wallpaper=""
if [ -f "$SET_SCRIPT" ]; then
    current_wallpaper=$(awk -F'"' '/mpvpaper/ {print $6}' "$SET_SCRIPT" 2>/dev/null)
    if [[ $current_wallpaper == \$HOME* ]]; then
        current_wallpaper="${current_wallpaper/\$HOME/$HOME}"
    fi
fi
[ -z "$current_wallpaper" ] && current_wallpaper=""

# // -- map display names to real files -- //
declare -A menu_map
menu_entries=" Add background\n"

# // -- build menu entries + add checkmark to current one -- //
for file in "$WALLPAPER_DIR"/*; do
    [ -f "$file" ] || continue

    ext="${file##*.}"
    base=$(basename "$file")
    name="${base%.*}"

    case "$ext" in
        jpg|png|jpeg) icon=" " ;;
        mp4|gif)      icon=" " ;;
        *)            icon=""   ;;
    esac

    display="$icon $name"

    if [ "$file" = "$current_wallpaper" ]; then
        display="$display ✓"
    fi

    menu_map["$display"]="$file"
    menu_entries+="$display\n"
done

#  // -- Rofi menu configuration -- //
chosen=$(echo -e "$menu_entries" | rofi -dmenu -i -selected-row 1 \
    -theme-str '
        textbox-prompt-colon {
            str: " Background:";
        }
        entry {
            placeholder: "";
        }
    '
)

# // -- handle Add Wallpaper option -- //
if [ "$chosen" = " Add background" ]; then
    nautilus "$WALLPAPER_DIR" &
    exit 0
fi

# // -- set wallpaper if something was selected -- //
if [ -n "$chosen" ] && [ -n "${menu_map[$chosen]}" ]; then
    selected_file="${menu_map[$chosen]}"
    [ -f "$selected_file" ] || exit 1

    relative_path="${selected_file/#$HOME/\$HOME}"

    killall mpvpaper 2>/dev/null
    mpvpaper -o "$MPV_OPTIONS" "*" "$selected_file" &

    # // overwrite set-wallpaper.sh cleanly -- //
    cat << EOF > "$SET_SCRIPT"
#!/bin/bash
mpvpaper -o "$MPV_OPTIONS" "*" "$relative_path"
EOF
    chmod +x "$SET_SCRIPT"
fi
