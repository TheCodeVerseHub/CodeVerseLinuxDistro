#!/bin/bash
# CVH Linux Installer - Detection Module
# System detection functions

# Detect boot mode (UEFI or BIOS)
detect_boot_mode() {
    step_header "Detecting System"

    gum spin --spinner dot --title "Checking boot mode..." -- sleep 0.5

    if [[ -d /sys/firmware/efi/efivars ]]; then
        BOOT_MODE="uefi"
        gum style --foreground 82 "  ✓ Boot mode: UEFI"
    else
        BOOT_MODE="bios"
        gum style --foreground 208 "  ● Boot mode: BIOS/Legacy"
    fi
}

# List available disks
list_disks() {
    lsblk -dno NAME,SIZE,MODEL | grep -vE "^(loop|sr|rom|fd|zram)"
}

# Get disk names only
get_disk_names() {
    lsblk -dno NAME | grep -vE "^(loop|sr|rom|fd|zram)"
}

# Get partition name based on disk type
get_partition_name() {
    local disk=$1
    local part_num=$2

    if [[ "$disk" == *"nvme"* ]] || [[ "$disk" == *"mmcblk"* ]]; then
        echo "${disk}p${part_num}"
    else
        echo "${disk}${part_num}"
    fi
}

# Check network connectivity
check_network() {
    if ! gum spin --spinner dot --title "Checking network..." -- ping -c 1 -W 5 archlinux.org; then
        gum style --foreground 208 "  ● Network: not connected"
        gum style --faint "  Attempting to connect..."

        systemctl start NetworkManager 2>/dev/null || true
        sleep 3

        for iface in $(ip -o link show | awk -F': ' '{print $2}' | grep -v lo); do
            dhcpcd "$iface" 2>/dev/null &
        done
        sleep 5

        if ! ping -c 1 -W 5 archlinux.org &>/dev/null; then
            gum style --foreground 196 "  ✗ No network connection"
            gum style --faint "    Use 'nmtui' or 'nmcli' to configure network"
            return 1
        fi
    fi
    gum style --foreground 82 "  ✓ Network: connected"
    return 0
}
