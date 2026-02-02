#!/bin/bash
# CVH Linux ISO Build Script
# Builds a bootable live ISO of CVH Linux

set -euo pipefail

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PROFILE_DIR="$PROJECT_ROOT/iso"
WORK_DIR="${WORK_DIR:-/tmp/cvh-build}"
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
    local theme_src="$PROJECT_ROOT/configs/grub/themes/cvh-nordic"
    local theme_iso="$PROJECT_ROOT/iso/grub/themes/cvh-nordic"

    if [[ -d "$theme_src" ]]; then
        log_info "Syncing GRUB theme..."
        mkdir -p "$theme_iso"
        cp -r "$theme_src"/* "$theme_iso/"
        log_success "GRUB theme synced to ISO profile"
    else
        log_warn "GRUB theme not found at $theme_src"
    fi

    # Sync niri config from configs to ISO skel (including wallpapers)
    local niri_src="$PROJECT_ROOT/configs/setup-configs/niri"
    local niri_dest="$WORK_DIR/profile/airootfs/etc/skel/.config/niri"

    if [[ -d "$niri_src" ]]; then
        log_info "Syncing niri configuration..."
        mkdir -p "$niri_dest"
        cp -r "$niri_src"/* "$niri_dest/"
        # Make scripts executable
        chmod +x "$niri_dest/scripts/"*.sh 2>/dev/null || true
        log_success "Niri config synced to ISO profile"
    else
        log_warn "Niri config not found at $niri_src"
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
    echo "  WORK_DIR         Build work directory (default: /tmp/cvh-build)"
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
