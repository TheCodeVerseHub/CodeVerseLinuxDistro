#!/bin/bash
# CVH Linux Installer - Input Module
# User input and selection functions

# Select keyboard layout
select_keyboard() {
    step_header "Keyboard Layout"

    local kb_options=(
        "us - US English"
        "uk - UK English"
        "de - German"
        "fr - French"
        "es - Spanish"
        "il - Hebrew"
        "Other"
    )

    local selection
    selection=$(gum choose --header "Select keyboard layout" --cursor.foreground="6" "${kb_options[@]}")

    if [[ "$selection" == "Other" ]]; then
        KEYMAP=$(gum input --placeholder "Enter keymap name (e.g., br, it, ru)")
        [[ -z "$KEYMAP" ]] && KEYMAP="us"
    else
        KEYMAP="${selection%% *}"
    fi

    loadkeys "$KEYMAP" 2>/dev/null || true
    echo -e "\n  ${GREEN}✓${NC} Keyboard: ${BOLD}$KEYMAP${NC}"
}

# Select timezone
select_timezone() {
    step_header "Timezone"
    TIMEZONE=$(timedatectl list-timezones | gum filter --height 20 --header "Select a timezone") || exit 1
    echo -e "\n  ${GREEN}✓${NC} Timezone: ${BOLD}$TIMEZONE${NC}"
}

# Select compositor
select_compositor() {
    step_header "Compositor Selection"

    local comp_options=(
        "niri - Scrollable-tiling compositor"
        "hyprland - Dynamic tiling compositor"
    )

    local selection
    selection=$(gum choose --header "Select Wayland compositor" --cursor.foreground="6" "${comp_options[@]}")

    COMPOSITOR="${selection%% *}"

    echo -e "\n  ${GREEN}✓${NC} Compositor: ${BOLD}$COMPOSITOR${NC}"
}

# Select disk for installation
select_disk() {
    step_header "Disk Selection"

    # Build disk options array
    local disk_options=()
    while IFS= read -r line; do
        local name=$(echo "$line" | awk '{print $1}')
        local size=$(echo "$line" | awk '{print $2}')
        local model=$(echo "$line" | awk '{$1=$2=""; print $0}' | xargs)
        disk_options+=("/dev/$name ($size) $model")
    done < <(list_disks)

    if [[ ${#disk_options[@]} -eq 0 ]]; then
        log_error "No suitable disks found!"
        exit 1
    fi

    local selection
    selection=$(gum choose --header "Select installation disk" --cursor.foreground="6" "${disk_options[@]}")

    # Extract disk path from selection
    DISK=$(echo "$selection" | awk '{print $1}')

    echo
    gum style --foreground 208 --bold "⚠  Selected: $DISK"
    gum style --foreground 196 "   ALL DATA WILL BE DESTROYED!"
    echo

    if ! gum confirm "Are you sure you want to format $DISK?"; then
        log_error "Installation cancelled"
        exit 1
    fi

    echo -e "\n  ${GREEN}✓${NC} Disk: ${BOLD}$DISK${NC}"
}

# Set hostname
set_hostname() {
    step_header "System Configuration"

    HOSTNAME=$(gum input --placeholder "cvh-linux" --header "Enter hostname" --value "cvh-linux")
    HOSTNAME=${HOSTNAME:-cvh-linux}

    if [[ ! "$HOSTNAME" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?$ ]]; then
        log_warn "Invalid hostname, using: cvh-linux"
        HOSTNAME="cvh-linux"
    fi

    echo -e "  ${GREEN}✓${NC} Hostname: ${BOLD}$HOSTNAME${NC}"
}

# Create user account configuration
create_user_config() {
    echo

    USERNAME=$(gum input --placeholder "cvh" --header "Enter username" --value "cvh")
    USERNAME=${USERNAME:-cvh}

    if [[ ! "$USERNAME" =~ ^[a-z_][a-z0-9_-]*$ ]]; then
        log_warn "Invalid username, using: cvh"
        USERNAME="cvh"
    fi

    echo -e "  ${GREEN}✓${NC} Username: ${BOLD}$USERNAME${NC}"
}
