#!/bin/bash
# CVH Linux Installer - Disk Module
# Disk partitioning and formatting functions

# Partition the disk
partition_disk() {
    step_header "Partitioning Disk"

    gum style --faint "  Preparing disk..."

    # Wipe existing partition table
    gum spin --spinner dot --title "Wiping partition table..." -- bash -c "wipefs -af '$DISK' >/dev/null 2>&1 || true; sgdisk -Z '$DISK' >/dev/null 2>&1 || true"

    if [[ "$BOOT_MODE" == "uefi" ]]; then
        partition_disk_uefi
    else
        partition_disk_bios
    fi

    gum style --foreground 82 "  ✓ Disk prepared successfully"
}

# Partition disk for UEFI
partition_disk_uefi() {
    gum style --foreground 6 "  ● Creating GPT partition table (UEFI)"

    gum spin --spinner dot --title "Creating GPT table..." -- parted -s "$DISK" mklabel gpt

    gum spin --spinner dot --title "Creating EFI partition..." -- bash -c "parted -s '$DISK' mkpart primary fat32 1MiB 513MiB; parted -s '$DISK' set 1 esp on"

    gum spin --spinner dot --title "Creating root partition..." -- parted -s "$DISK" mkpart primary ext4 513MiB 100%

    EFI_PART=$(get_partition_name "$DISK" 1)
    ROOT_PART=$(get_partition_name "$DISK" 2)

    sleep 1  # Wait for kernel to recognize partitions

    gum style --foreground 6 "  ● Formatting EFI partition (FAT32)"
    gum spin --spinner dot --title "Formatting EFI..." -- mkfs.fat -F32 "$EFI_PART"

    gum style --foreground 6 "  ● Formatting root partition (ext4)"
    gum spin --spinner dot --title "Formatting root..." -- mkfs.ext4 -F "$ROOT_PART"

    gum style --foreground 6 "  ● Mounting partitions"
    mount "$ROOT_PART" /mnt
    mkdir -p /mnt/boot/efi
    mount "$EFI_PART" /mnt/boot/efi
}

# Partition disk for BIOS
partition_disk_bios() {
    gum style --foreground 6 "  ● Creating MBR partition table (BIOS)"

    gum spin --spinner dot --title "Creating MBR table..." -- parted -s "$DISK" mklabel msdos

    gum spin --spinner dot --title "Creating boot partition..." -- bash -c "parted -s '$DISK' mkpart primary ext4 1MiB 100%; parted -s '$DISK' set 1 boot on"

    ROOT_PART=$(get_partition_name "$DISK" 1)

    sleep 1

    gum style --foreground 6 "  ● Formatting root partition (ext4)"
    gum spin --spinner dot --title "Formatting root..." -- mkfs.ext4 -F "$ROOT_PART"

    gum style --foreground 6 "  ● Mounting partition"
    mount "$ROOT_PART" /mnt
}

# Generate fstab
generate_fstab() {
    step_header "Generating Filesystem Table"

    gum spin --spinner dot --title "Creating /etc/fstab..." -- bash -c "genfstab -U /mnt >> /mnt/etc/fstab"

    gum style --foreground 82 "  ✓ Filesystem table generated"
}

# Unmount all partitions
unmount_all() {
    gum spin --spinner dot --title "Unmounting partitions..." -- umount -R /mnt
    gum style --foreground 82 "  ✓ Partitions unmounted"
}
