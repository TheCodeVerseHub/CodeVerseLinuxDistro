#!/usr/bin/env bash
# CVH Linux ISO Profile Definition
# Based on archiso profile format

iso_name="cvh-linux"
iso_label="CVH_LINUX_$(date +%Y%m)"
iso_publisher="CodeVerse Hub <https://codeversehub.dev>"
iso_application="CVH Linux Live/Install ISO"
iso_version="$(date +%Y.%m.%d)"
install_dir="cvh"
buildmodes=('iso')

# Boot modes:
# - bios.syslinux.mbr: BIOS boot from MBR
# - bios.syslinux.eltorito: BIOS boot from CD/DVD
# - uefi-x64.grub.esp: UEFI boot from ESP partition
# - uefi-x64.grub.eltorito: UEFI boot from CD/DVD
bootmodes=('bios.syslinux.mbr' 'bios.syslinux.eltorito' 'uefi-x64.grub.esp' 'uefi-x64.grub.eltorito')

arch="x86_64"
pacman_conf="pacman.conf"
airootfs_image_type="squashfs"
airootfs_image_tool_options=('-comp' 'zstd' '-Xcompression-level' '15')

# File permissions for airootfs
file_permissions=(
    ["/etc/shadow"]="0:0:400"
    ["/etc/gshadow"]="0:0:400"
    ["/etc/passwd"]="0:0:644"
    ["/etc/group"]="0:0:644"
    ["/etc/sudoers.d"]="0:0:750"
    ["/root"]="0:0:750"
    ["/root/.zshrc"]="0:0:644"
    ["/usr/bin/cvh-install"]="0:0:755"
)
