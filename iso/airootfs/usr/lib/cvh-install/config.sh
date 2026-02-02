#!/bin/bash
# CVH Linux Installer - Configuration Module
# Global variables and defaults

# Progress tracking
export TOTAL_STEPS=10
export CURRENT_STEP=0

# Installation target variables
export DISK=""
export BOOT_MODE=""
export EFI_PART=""
export ROOT_PART=""

# User configuration with defaults
export USERNAME="cvh"
export HOSTNAME="cvh-linux"
export TIMEZONE="Asia/Jerusalem"
export LOCALE="en_US.UTF-8"
export KEYMAP="us"
export COMPOSITOR="niri"  # CVH Linux uses niri compositor

# Package lists
get_base_packages() {
    echo "base base-devel"
    echo "linux linux-firmware linux-headers"
    echo "grub efibootmgr"
    echo "networkmanager"
    echo "zsh zsh-completions git"
    echo "pipewire pipewire-pulse wireplumber"
    echo "noto-fonts noto-fonts-emoji ttf-dejavu ttf-liberation ttf-fira-code"
    echo "sudo nano vim"
    echo "seatd"
    echo "ly"
    echo "gcc pkgconf"
}

get_shell_utils() {
    echo "gum zoxide fd ripgrep bat eza"
}

get_system_utils() {
    echo "htop btop tree fastfetch nnn"
}

get_sandbox_packages() {
    echo "bubblewrap libseccomp"
}

get_wayland_packages() {
    echo "foot mako fuzzel"
    echo "grim slurp wl-clipboard"
    echo "wayland wayland-protocols xorg-xwayland"
    echo "brightnessctl"
}

get_niri_packages() {
    echo "niri"
    echo "xdg-desktop-portal-gnome"
    echo "xdg-desktop-portal-gtk"
}

# Get all packages for installation
get_all_packages() {
    get_base_packages
    get_shell_utils
    get_system_utils
    get_sandbox_packages
    get_wayland_packages
    get_niri_packages
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This installer must be run as root"
        exit 1
    fi
}
