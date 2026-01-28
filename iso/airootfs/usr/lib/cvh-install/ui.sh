#!/bin/bash
# CVH Linux Installer - UI Module

# Logging functions using gum
log_info() { gum log --level info "$1"; }
log_success() { gum log --level info --prefix "✓" "$1"; }
log_warn() { gum log --level warn "$1"; }
log_error() { gum log --level error "$1"; }

# Usage: run_with_spinner "message" command args...
run_with_spinner() {
    local message="$1"
    shift
    if gum spin --spinner dot --title "$message" -- "$@"; then
        gum log --level info --prefix "✓" "$message"
    else
        gum log --level error --prefix "✗" "$message"
        return 1
    fi
}

step_header() {
    ((CURRENT_STEP++))
    echo
    gum style \
        --foreground 6 --border-foreground 6 --border rounded \
        --padding "0 2" --margin "0 0" \
        "Step ${CURRENT_STEP}/${TOTAL_STEPS}: $1"
    echo
}

show_overall_progress() {
    local percent=$((CURRENT_STEP * 100 / TOTAL_STEPS))
    echo
    gum style --faint "Overall Progress: ${percent}%"
}

show_welcome() {
    clear

    gum style --foreground 6 --bold '
   ██████╗██╗   ██╗██╗  ██╗    ██╗     ██╗███╗   ██╗██╗   ██╗██╗  ██╗
  ██╔════╝██║   ██║██║  ██║    ██║     ██║████╗  ██║██║   ██║╚██╗██╔╝
  ██║     ██║   ██║███████║    ██║     ██║██╔██╗ ██║██║   ██║ ╚███╔╝
  ██║     ╚██╗ ██╔╝██╔══██║    ██║     ██║██║╚██╗██║██║   ██║ ██╔██╗
  ╚██████╗ ╚████╔╝ ██║  ██║    ███████╗██║██║ ╚████║╚██████╔╝██╔╝ ██╗
   ╚═════╝  ╚═══╝  ╚═╝  ╚═╝    ╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝'

    gum style --faint --align center "CodeVerse Hub Linux Distribution"
    echo

    gum style --bold "Features:"
    gum style "  • Niri or Hyprland Wayland compositor"
    gum style "  • Ly display manager"
    gum style "  • Zsh + Oh My Zsh"
    gum style "  • Custom fuzzy finder & icon system"
    gum style "  • Minimal & lightweight"
    echo

    gum style --foreground 208 --bold "⚠  WARNING: This will ERASE all data on the selected disk!"
    echo

    if ! gum confirm "Begin installation?" --affirmative "Start" --negative "Cancel"; then
        gum style --foreground 196 "Installation cancelled"
        exit 0
    fi
}

show_completion() {
    echo
    gum style \
        --foreground 82 --border-foreground 82 --border double \
        --align center --width 60 --margin "1 2" --padding "1 4" \
        "✓ Installation Complete!" "" \
        "CVH Linux has been installed successfully."
}
