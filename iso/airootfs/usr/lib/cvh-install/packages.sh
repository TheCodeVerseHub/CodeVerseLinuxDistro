#!/bin/bash
# CVH Linux Installer - Packages Module
# Package installation functions

# Install base system
install_base() {
    step_header "Installing Base System"

    # Check network connectivity
    if ! check_network; then
        exit 1
    fi

    # Initialize pacman keyring
    gum style --foreground 6 "  ● Initializing package keyring..."
    gum spin --spinner dot --title "Initializing keyring..." -- bash -c "pacman-key --init >/dev/null 2>&1; pacman-key --populate archlinux >/dev/null 2>&1"
    gum style --foreground 82 "  ✓ Keyring initialized"

    # Build package list
    local packages=()
    while IFS= read -r pkg; do
        [[ -n "$pkg" ]] && packages+=($pkg)
    done < <(get_all_packages)

    echo
    gum style --foreground 6 "  ● Installing packages (this may take a while)..."
    gum style --faint "  ────────────────────────────────────────────────────────"
    echo

    # Run pacstrap - use host cache (-c) to avoid re-downloading packages already in ISO
    if pacstrap -c -K /mnt "${packages[@]}"; then
        echo
        gum style --faint "  ────────────────────────────────────────────────────────"
        gum style --foreground 82 "  ✓ Base system installed"
    else
        echo
        gum style --foreground 196 "  ✗ Package installation failed!"
        exit 1
    fi
}

# Copy CVH custom packages from ISO
copy_cvh_packages() {
    gum style --foreground 6 "  ● Copying CVH custom packages from ISO..."

    mkdir -p /mnt/var/cache/pacman/cvh-packages
    if [[ -d /opt/cvh-repo ]] && ls /opt/cvh-repo/*.pkg.tar.zst >/dev/null 2>&1; then
        cp /opt/cvh-repo/*.pkg.tar.zst /mnt/var/cache/pacman/cvh-packages/ 2>/dev/null || true
        local count=$(ls /opt/cvh-repo/*.pkg.tar.zst 2>/dev/null | wc -l)
        gum style --foreground 82 "  ✓ CVH packages copied ($count packages)"
    else
        gum style --foreground 208 "  ⚠ CVH packages not found on ISO"
    fi
}

# Create mirrorlist for installed system
create_mirrorlist() {
    gum style --foreground 6 "  ● Creating package mirrorlist..."
    mkdir -p /mnt/etc/pacman.d
    cat > /mnt/etc/pacman.d/mirrorlist << 'EOF'
# Arch Linux mirrorlist - CVH Linux
# Israeli mirrors
Server = https://mirror.isoc.org.il/pub/archlinux/$repo/os/$arch
Server = https://archlinux.mivzakim.net/$repo/os/$arch
# Global mirrors
Server = https://geo.mirror.pkgbuild.com/$repo/os/$arch
Server = https://mirrors.kernel.org/archlinux/$repo/os/$arch
Server = https://mirror.rackspace.com/archlinux/$repo/os/$arch
EOF
    gum style --foreground 82 "  ✓ Mirrorlist created"
}

# Configure pacman repositories
configure_pacman_repos() {
    gum style --foreground 6 "  ● Configuring package repositories..."
    if [[ -f /mnt/etc/pacman.conf ]]; then
        # Check if repos are already configured
        if ! grep -q "^\[core\]" /mnt/etc/pacman.conf; then
            cat >> /mnt/etc/pacman.conf << 'EOF'

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
EOF
            gum style --foreground 82 "  ✓ Repositories configured"
        else
            gum style --foreground 82 "  ✓ Repositories already configured"
        fi
    fi
}
