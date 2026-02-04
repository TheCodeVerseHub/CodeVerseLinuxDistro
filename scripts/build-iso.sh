#!/bin/bash
# CVH Linux ISO Build Script
# Builds a bootable live ISO of CVH Linux

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROFILE_DIR="$PROJECT_ROOT/iso"
WORK_DIR="${WORK_DIR:-$PROJECT_ROOT/.build}"
OUT_DIR="$PROJECT_ROOT/out"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Check dependencies
check_dependencies() {
    log_info "Checking dependencies..."

    local deps=(
        "mkarchiso"
        "mksquashfs"
        "xorriso"
        "grub-mkrescue"
    )

    local missing=()
    for dep in "${deps[@]}"; do
        if ! command -v "$dep" &> /dev/null; then
            missing+=("$dep")
        fi
    done

    if [[ ${#missing[@]} -gt 0 ]]; then
        log_error "Missing dependencies: ${missing[*]}"
        log_info "Install with: sudo pacman -S archiso squashfs-tools libisoburn grub"
        exit 1
    fi

    log_success "All dependencies found"
}

# Build custom packages
build_packages() {
    log_info "Building custom packages..."

    "$SCRIPT_DIR/build-packages.sh"

    log_success "Custom packages built"
}

# Prepare the profile
prepare_profile() {
    log_info "Preparing ISO profile..."

    # Create work directory
    mkdir -p "$WORK_DIR" "$OUT_DIR"

    # Copy profile to work directory
    rm -rf "$WORK_DIR/profile"
    cp -r "$PROFILE_DIR" "$WORK_DIR/profile"

    # Sync GRUB theme from configs to iso directory
    local theme_src="$PROJECT_ROOT/configs/grub/themes/tela"
    local theme_iso="$PROJECT_ROOT/iso/grub/themes/tela"

    if [[ -d "$theme_src" ]]; then
        log_info "Syncing GRUB theme..."
        mkdir -p "$theme_iso"
        cp -r "$theme_src"/* "$theme_iso/"
        log_success "GRUB theme synced to ISO profile"
    else
        log_warn "GRUB theme not found at $theme_src"
    fi

    # Sync all setup configs from configs/setup-configs to ISO skel
    local setup_src="$PROJECT_ROOT/configs/setup-configs"
    local skel_dest="$WORK_DIR/profile/airootfs/etc/skel"

    if [[ -d "$setup_src" ]]; then
        log_info "Syncing setup configurations to ISO skel..."

        # Niri config -> ~/.config/niri
        if [[ -d "$setup_src/niri" ]]; then
            mkdir -p "$skel_dest/.config/niri"
            cp -r "$setup_src/niri"/* "$skel_dest/.config/niri/"
            chmod +x "$skel_dest/.config/niri/scripts/"*.sh 2>/dev/null || true
            log_success "  Niri config synced"
        fi

        # GTK configs -> ~/.config/gtk-3.0 and ~/.config/gtk-4.0
        if [[ -d "$setup_src/gtk-configs" ]]; then
            for gtk_ver in gtk-3.0 gtk-4.0; do
                if [[ -d "$setup_src/gtk-configs/$gtk_ver" ]]; then
                    mkdir -p "$skel_dest/.config/$gtk_ver"
                    cp -r "$setup_src/gtk-configs/$gtk_ver"/* "$skel_dest/.config/$gtk_ver/"
                fi
            done
            log_success "  GTK configs synced"
        fi

        # Kitty config -> ~/.config/kitty
        if [[ -d "$setup_src/kitty" ]]; then
            mkdir -p "$skel_dest/.config/kitty"
            cp -r "$setup_src/kitty"/* "$skel_dest/.config/kitty/"
            log_success "  Kitty config synced"
        fi

        # Rofi config -> ~/.config/rofi
        if [[ -d "$setup_src/rofi" ]]; then
            mkdir -p "$skel_dest/.config/rofi"
            cp -r "$setup_src/rofi"/* "$skel_dest/.config/rofi/"
            chmod +x "$skel_dest/.config/rofi/scripts/"*.sh 2>/dev/null || true
            log_success "  Rofi config synced"
        fi

        # Swaync config -> ~/.config/swaync
        if [[ -d "$setup_src/swaync" ]]; then
            mkdir -p "$skel_dest/.config/swaync"
            cp -r "$setup_src/swaync"/* "$skel_dest/.config/swaync/"
            log_success "  Swaync config synced"
        fi

        # SwayOSD config -> ~/.config/swayosd
        if [[ -d "$setup_src/swayosd" ]]; then
            mkdir -p "$skel_dest/.config/swayosd"
            cp -r "$setup_src/swayosd"/* "$skel_dest/.config/swayosd/"
            log_success "  SwayOSD config synced"
        fi

        # Waybar config -> ~/.config/waybar
        if [[ -d "$setup_src/waybar" ]]; then
            mkdir -p "$skel_dest/.config/waybar"
            cp -r "$setup_src/waybar"/* "$skel_dest/.config/waybar/"
            log_success "  Waybar config synced"
        fi

        # Note: zsh/.zshrc is NOT synced - configure.sh creates a proper .zshrc
        # with Wayland env vars and compositor auto-start

        # Themes and icons -> ~/.icons and ~/.themes
        if [[ -d "$setup_src/themes-and-icons" ]]; then
            if [[ -d "$setup_src/themes-and-icons/.icons" ]]; then
                mkdir -p "$skel_dest/.icons"
                cp -r "$setup_src/themes-and-icons/.icons"/* "$skel_dest/.icons/"
                log_success "  Icons synced"
            fi
            if [[ -d "$setup_src/themes-and-icons/.themes" ]]; then
                mkdir -p "$skel_dest/.themes"
                cp -r "$setup_src/themes-and-icons/.themes"/* "$skel_dest/.themes/"
                log_success "  Themes synced"
            fi
        fi

        log_success "All setup configs synced to ISO profile"
    else
        log_warn "Setup configs not found at $setup_src"
    fi

    # Copy built packages (CVH + AUR) to ISO for offline installation
    local repo_src="$PROJECT_ROOT/repo/x86_64"
    local repo_dest="$WORK_DIR/profile/airootfs/opt/cvh-repo"

    if [[ -d "$repo_src" ]] && ls "$repo_src"/*.pkg.tar.zst >/dev/null 2>&1; then
        log_info "Copying built packages to ISO..."
        mkdir -p "$repo_dest"
        cp "$repo_src"/*.pkg.tar.zst "$repo_dest/"
        local pkg_count=$(ls "$repo_dest"/*.pkg.tar.zst 2>/dev/null | wc -l)
        log_success "Copied $pkg_count packages to ISO"
    else
        log_warn "No built packages found at $repo_src"
    fi

    # Make scripts executable
    chmod +x "$WORK_DIR/profile/profiledef.sh"

    log_success "Profile prepared"
}

# Build the ISO
build_iso() {
    log_info "Building ISO..."
    log_info "This may take a while..."

    # Run mkarchiso
    sudo mkarchiso -v \
        -w "$WORK_DIR/work" \
        -o "$OUT_DIR" \
        "$WORK_DIR/profile"

    log_success "ISO built successfully!"

    # Find the built ISO
    local iso_file=$(ls -t "$OUT_DIR"/*.iso 2>/dev/null | head -1)
    if [[ -n "$iso_file" ]]; then
        log_success "ISO file: $iso_file"
        log_info "Size: $(du -h "$iso_file" | cut -f1)"
    fi
}

# Clean up
cleanup() {
    log_info "Cleaning up..."

    sudo rm -rf "$WORK_DIR/work"

    log_success "Cleanup complete"
}

# Show usage
usage() {
    echo "CVH Linux ISO Build Script"
    echo
    echo "Usage: $0 [OPTIONS]"
    echo
    echo "Options:"
    echo "  -h, --help       Show this help message"
    echo "  -c, --clean      Clean build directory before building"
    echo "  -p, --packages   Only build custom packages"
    echo "  -n, --no-pkg     Skip building custom packages"
    echo
    echo "Environment variables:"
    echo "  WORK_DIR         Build work directory (default: \$PROJECT_ROOT/.build)"
    echo
}

# Main
main() {
    local clean=false
    local packages_only=false
    local skip_packages=false

    while [[ $# -gt 0 ]]; do
        case $1 in
            -h|--help)
                usage
                exit 0
                ;;
            -c|--clean)
                clean=true
                shift
                ;;
            -p|--packages)
                packages_only=true
                shift
                ;;
            -n|--no-pkg)
                skip_packages=true
                shift
                ;;
            *)
                log_error "Unknown option: $1"
                usage
                exit 1
                ;;
        esac
    done

    echo
    echo "╔════════════════════════════════════════════╗"
    echo "║       CVH Linux ISO Build Script           ║"
    echo "╚════════════════════════════════════════════╝"
    echo

    check_dependencies

    if $clean; then
        log_info "Cleaning previous build..."
        sudo rm -rf "$WORK_DIR"
    fi

    if ! $skip_packages; then
        build_packages
    fi

    if $packages_only; then
        log_success "Packages built. Skipping ISO build."
        exit 0
    fi

    prepare_profile
    build_iso
    cleanup

    echo
    log_success "Build complete!"
    echo
    echo "To test the ISO with QEMU:"
    echo "  qemu-system-x86_64 -enable-kvm -m 4G -cdrom $OUT_DIR/cvh-linux-*.iso"
    echo
}

main "$@"
