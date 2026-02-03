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

# Package lists - synced with packages.x86_64
# Note: archiso-specific packages (mkinitcpio-archiso, nbd, etc.) are excluded

get_base_packages() {
    # Kernel and firmware
    echo "linux linux-firmware linux-headers"
    # Base system
    echo "base base-devel"
    # Init system
    echo "systemd dbus"
    # Bootloader
    echo "grub efibootmgr os-prober syslinux"
    # Filesystem utilities
    echo "dosfstools e2fsprogs btrfs-progs xfsprogs ntfs-3g mtools"
    # Networking
    echo "networkmanager iwd openssh curl wget rsync"
    # Hardware support
    echo "pciutils usbutils lshw"
    # mkinitcpio for installed system
    echo "mkinitcpio"
}

get_core_utils() {
    echo "coreutils util-linux procps-ng shadow"
    echo "grep sed gawk findutils diffutils"
    echo "file less which man-db man-pages"
    echo "libnotify reflector"
    # Compression
    echo "gzip bzip2 xz zstd lz4 zip unzip"
}

get_audio_packages() {
    echo "pipewire pipewire-pulse pipewire-alsa wireplumber"
}

get_wayland_packages() {
    # Wayland core
    echo "wayland wayland-protocols xorg-xwayland"
    echo "qt5-wayland qt6-wayland xwayland-satellite"
    # Wayland utilities
    echo "rofi grim slurp wl-clipboard"
    echo "swaync swayosd cliphist brightnessctl"
    echo "wf-recorder waybar"
    echo "polkit-gnome gnome-keyring"
    # Note: mpvpaper is AUR, installed via CVH repo packages
}

get_niri_packages() {
    echo "niri"
    echo "xdg-desktop-portal xdg-desktop-portal-gnome xdg-desktop-portal-gtk"
    echo "xdg-utils mesa vulkan-icd-loader seatd"
}

get_shell_packages() {
    # Shell utilities
    echo "gum zoxide fd ripgrep bat eza git"
    # Terminal and shell
    echo "zsh zsh-completions zsh-syntax-highlighting"
    echo "zsh-autosuggestions zsh-history-substring-search"
    # Terminal emulator
    echo "kitty"
}

get_fonts() {
    echo "noto-fonts noto-fonts-emoji"
    echo "ttf-jetbrains-mono-nerd ttf-nerd-fonts-symbols"
    echo "ttf-dejavu ttf-liberation ttf-fira-code"
}

get_system_utils() {
    echo "btop tree fastfetch"
    # File management
    echo "nautilus"
}

get_sandbox_packages() {
    echo "bubblewrap libseccomp"
}

get_development_packages() {
    echo "gcc pkgconf"
}

get_display_manager() {
    echo "ly"
}

get_default_apps() {
    echo "zed neovim"
    echo "celluloid transmission-gtk"
    echo "evince loupe file-roller"
    echo "nwg-look tangram"
}

# CVH custom packages - NOT included in pacstrap
# These are installed via pacman -U from /opt/cvh-repo after pacstrap
# Packages: cvh-fuzzy, cvh-icons, cvh-branding, mpvpaper (AUR)

# Get all packages for installation (official repos only)
# CVH packages (cvh-fuzzy, cvh-icons, cvh-branding, mpvpaper) are installed
# separately via pacman -U from /opt/cvh-repo after pacstrap
get_all_packages() {
    get_base_packages
    get_core_utils
    get_audio_packages
    get_wayland_packages
    get_niri_packages
    get_shell_packages
    get_fonts
    get_system_utils
    get_sandbox_packages
    get_development_packages
    get_display_manager
    get_default_apps
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This installer must be run as root"
        exit 1
    fi
}
