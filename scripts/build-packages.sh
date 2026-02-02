#!/bin/bash
# Build custom CVH Linux packages
# Creates cvh-fuzzy, cvh-icons, and cvh-branding packages

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PKGBUILD_DIR="$PROJECT_ROOT/pkgbuild"
SRC_DIR="$PROJECT_ROOT/src"
REPO_DIR="$PROJECT_ROOT/repo/x86_64"
GRUB_THEME_DIR="$PROJECT_ROOT/configs/grub/themes/cvh-nordic"

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

# Check for Rust
check_rust() {
    if ! command -v cargo &> /dev/null; then
        log_error "Rust/Cargo not found. Please install rustup."
        exit 1
    fi
    log_success "Rust toolchain found"
}

# Generate GRUB theme assets
generate_grub_theme() {
    log_info "Generating GRUB theme assets..."

    # Check for ImageMagick
    if ! command -v convert &> /dev/null; then
        log_warn "ImageMagick not found, skipping theme generation"
        log_warn "Install with: sudo pacman -S imagemagick"
        return 0
    fi

    mkdir -p "$GRUB_THEME_DIR/icons"

    # Nord colors
    local NORD0="#2E3440"  # Polar Night (darkest)
    local NORD1="#3B4252"  # Polar Night
    local NORD3="#4C566A"  # Polar Night (lightest)
    local NORD4="#D8DEE9"  # Snow Storm
    local NORD8="#88C0D0"  # Frost (cyan)

    # Generate background (1920x1080 with subtle gradient)
    log_info "  Creating background.png..."
    convert -size 1920x1080 \
        -define gradient:direction=south \
        gradient:"$NORD0"-"$NORD1" \
        "$GRUB_THEME_DIR/background.png"

    # Generate menu icons (32x32)
    log_info "  Creating menu icons..."

    # Linux icon (simple penguin silhouette approximation)
    convert -size 32x32 xc:transparent \
        -fill "$NORD4" \
        -draw "circle 16,12 16,4" \
        -draw "roundrectangle 8,14 24,28 4,4" \
        "$GRUB_THEME_DIR/icons/linux.png"

    # CVH/Arch icon
    convert -size 32x32 xc:transparent \
        -fill "$NORD8" \
        -draw "polygon 16,4 28,28 4,28" \
        -fill "$NORD0" \
        -draw "polygon 16,14 22,24 10,24" \
        "$GRUB_THEME_DIR/icons/arch.png"

    cp "$GRUB_THEME_DIR/icons/arch.png" "$GRUB_THEME_DIR/icons/cvh.png"

    # Reboot icon (circular arrow)
    convert -size 32x32 xc:transparent \
        -fill "$NORD4" \
        -stroke "$NORD4" -strokewidth 3 \
        -draw "arc 6,6 26,26 30,330" \
        -draw "polygon 24,4 28,10 22,10" \
        "$GRUB_THEME_DIR/icons/reboot.png"

    # Shutdown icon (power symbol)
    convert -size 32x32 xc:transparent \
        -fill none -stroke "$NORD4" -strokewidth 3 \
        -draw "arc 8,10 24,26 -60,240" \
        -draw "line 16,6 16,16" \
        "$GRUB_THEME_DIR/icons/shutdown.png"

    # UEFI settings icon (gear)
    convert -size 32x32 xc:transparent \
        -fill "$NORD4" \
        -draw "circle 16,16 16,8" \
        -fill "$NORD0" \
        -draw "circle 16,16 16,12" \
        "$GRUB_THEME_DIR/icons/uefi.png"

    log_success "GRUB theme assets generated"
}

# Build cvh-fuzzy
build_cvh_fuzzy() {
    log_info "Building cvh-fuzzy..."

    cd "$SRC_DIR/cvh-fuzzy"

    # Build release binary
    cargo build --release

    log_success "cvh-fuzzy built"
}

# Build cvh-icons
build_cvh_icons() {
    log_info "Building cvh-icons..."

    cd "$SRC_DIR/cvh-icons"

    # Build release binary
    cargo build --release

    log_success "cvh-icons built"
}

# Create PKGBUILD for cvh-fuzzy
create_fuzzy_pkgbuild() {
    log_info "Creating cvh-fuzzy PKGBUILD..."

    mkdir -p "$PKGBUILD_DIR/cvh-fuzzy"

    # Use absolute path in PKGBUILD
    cat > "$PKGBUILD_DIR/cvh-fuzzy/PKGBUILD" <<EOF
# Maintainer: CVH Linux Team
pkgname=cvh-fuzzy
pkgver=0.1.0
pkgrel=1
pkgdesc="Universal fuzzy finder for CVH Linux"
arch=('x86_64')
url="https://github.com/codeversehub/cvh-linux"
license=('GPL3')
depends=('gcc-libs')
makedepends=('rust' 'cargo')
source=()

_cvh_root="$PROJECT_ROOT"

build() {
    cd "\$_cvh_root/src/cvh-fuzzy"
    cargo build --release
}

package() {
    cd "\$_cvh_root/src/cvh-fuzzy"
    install -Dm755 "target/release/cvh-fuzzy" "\$pkgdir/usr/bin/cvh-fuzzy"

    # Install shell integration
    install -Dm644 /dev/stdin "\$pkgdir/usr/share/cvh-fuzzy/shell/zsh.zsh" <<'ZSHEOF'
# CVH Fuzzy Zsh Integration
if command -v cvh-fuzzy &> /dev/null; then
    cvh-fuzzy-file-widget() {
        local selected=\$(cvh-fuzzy --mode files)
        LBUFFER="\${LBUFFER}\${selected}"
        zle redisplay
    }
    zle -N cvh-fuzzy-file-widget
    bindkey '^T' cvh-fuzzy-file-widget

    cvh-fuzzy-history-widget() {
        local selected=\$(fc -rl 1 | cvh-fuzzy --mode stdin | sed 's/^ *[0-9]* *//')
        LBUFFER="\$selected"
        zle redisplay
    }
    zle -N cvh-fuzzy-history-widget
    bindkey '^R' cvh-fuzzy-history-widget
fi
ZSHEOF
}
EOF

    log_success "cvh-fuzzy PKGBUILD created"
}

# Create PKGBUILD for cvh-icons
create_icons_pkgbuild() {
    log_info "Creating cvh-icons PKGBUILD..."

    mkdir -p "$PKGBUILD_DIR/cvh-icons"

    # Use absolute path in PKGBUILD
    cat > "$PKGBUILD_DIR/cvh-icons/PKGBUILD" <<EOF
# Maintainer: CVH Linux Team
pkgname=cvh-icons
pkgver=0.1.0
pkgrel=1
pkgdesc="Sandboxed Lua-scriptable desktop icons for CVH Linux"
arch=('x86_64')
url="https://github.com/codeversehub/cvh-linux"
license=('GPL3')
depends=('gcc-libs' 'wayland' 'bubblewrap' 'lua')
makedepends=('rust' 'cargo')
source=()

_cvh_root="$PROJECT_ROOT"

build() {
    cd "\$_cvh_root/src/cvh-icons"
    cargo build --release
}

package() {
    cd "\$_cvh_root/src/cvh-icons"
    install -Dm755 "target/release/cvh-icons" "\$pkgdir/usr/bin/cvh-icons"

    # Install Lua scripts
    install -dm755 "\$pkgdir/usr/share/cvh-icons/scripts"
    install -Dm644 lua/widgets/*.lua "\$pkgdir/usr/share/cvh-icons/scripts/" 2>/dev/null || true

    # Install default config
    install -Dm644 /dev/stdin "\$pkgdir/etc/cvh-icons/config.toml" <<'CONFEOF'
# CVH Icons Configuration
icon_size = 64
grid_spacing = 20
font_size = 12.0
icon_theme = "Adwaita"

[sandbox]
enabled = true
allow_network = false

[colors]
label_fg = "#ffffff"
label_bg = "#00000080"
selection = "#88c0d040"
CONFEOF
}
EOF

    log_success "cvh-icons PKGBUILD created"
}

# Create PKGBUILD for cvh-branding
create_branding_pkgbuild() {
    log_info "Creating cvh-branding PKGBUILD..."

    mkdir -p "$PKGBUILD_DIR/cvh-branding"

    cat > "$PKGBUILD_DIR/cvh-branding/PKGBUILD" <<EOF
# Maintainer: CVH Linux Team
pkgname=cvh-branding
pkgver=0.1.0
pkgrel=1
pkgdesc="CVH Linux branding, GRUB theme, and default configurations"
arch=('any')
url="https://github.com/codeversehub/cvh-linux"
license=('GPL3')
depends=()
source=()

_cvh_root="$PROJECT_ROOT"

package() {
    # MOTD - welcome message
    install -Dm644 /dev/stdin "\$pkgdir/etc/motd" <<'MOTDEOF'
Welcome to CVH Linux!

Quick Start:
  - Mod+Return    Open terminal
  - Mod+D         Application launcher
  - Mod+1-9       Switch workspaces
  - Mod+Shift+Q   Close window
  - Mod+Shift+E   Exit compositor

For more info: https://github.com/codeversehub/cvh-linux
MOTDEOF

    # CVH Linux info file
    install -Dm644 /dev/stdin "\$pkgdir/usr/share/cvh-linux/info" <<'INFOEOF'
NAME="CVH Linux"
PRETTY_NAME="CVH Linux"
ID=cvh
VERSION_ID=0.1
HOME_URL="https://codeversehub.dev"
DOCUMENTATION_URL="https://github.com/codeversehub/cvh-linux"
INFOEOF

    # Install GRUB theme
    install -dm755 "\$pkgdir/usr/share/cvh-linux/grub-theme/cvh-nordic"
    install -dm755 "\$pkgdir/usr/share/cvh-linux/grub-theme/cvh-nordic/icons"

    # Copy theme files
    if [[ -f "\$_cvh_root/configs/grub/themes/cvh-nordic/theme.txt" ]]; then
        install -Dm644 "\$_cvh_root/configs/grub/themes/cvh-nordic/theme.txt" \\
            "\$pkgdir/usr/share/cvh-linux/grub-theme/cvh-nordic/theme.txt"
    fi

    if [[ -f "\$_cvh_root/configs/grub/themes/cvh-nordic/background.png" ]]; then
        install -Dm644 "\$_cvh_root/configs/grub/themes/cvh-nordic/background.png" \\
            "\$pkgdir/usr/share/cvh-linux/grub-theme/cvh-nordic/background.png"
    fi

    # Copy icons
    for icon in "\$_cvh_root/configs/grub/themes/cvh-nordic/icons"/*.png; do
        if [[ -f "\$icon" ]]; then
            install -Dm644 "\$icon" "\$pkgdir/usr/share/cvh-linux/grub-theme/cvh-nordic/icons/\$(basename \$icon)"
        fi
    done
}
EOF

    log_success "cvh-branding PKGBUILD created"
}

# Build an AUR package
build_aur_package() {
    local pkgname="$1"
    local aur_dir="$PKGBUILD_DIR/aur"

    log_info "Building AUR package: $pkgname"

    mkdir -p "$aur_dir"
    cd "$aur_dir"

    # Remove old clone if exists
    rm -rf "$pkgname"

    # Clone from AUR
    if ! git clone --depth=1 "https://aur.archlinux.org/${pkgname}.git" 2>/dev/null; then
        log_warn "Failed to clone $pkgname from AUR"
        return 1
    fi

    cd "$pkgname"

    # Build package (without installing)
    if makepkg -sf --noconfirm; then
        # Copy to repo
        cp -f *.pkg.tar.zst "$REPO_DIR/" 2>/dev/null || true
        log_success "AUR package $pkgname built"
        return 0
    else
        log_warn "Failed to build AUR package $pkgname"
        return 1
    fi
}

# Build all AUR packages
build_aur_packages() {
    log_info "Building AUR packages..."

    local aur_packages=(
        "mpvpaper"
        "fzf-zsh-plugin"
    )

    for pkg in "${aur_packages[@]}"; do
        build_aur_package "$pkg" || true
    done

    log_success "AUR packages processed"
}

# Build all packages
build_all_packages() {
    mkdir -p "$REPO_DIR"

    for pkg in cvh-fuzzy cvh-icons cvh-branding; do
        log_info "Building package: $pkg"

        cd "$PKGBUILD_DIR/$pkg"

        # Clean previous builds
        rm -f *.pkg.tar.zst

        # Build package
        makepkg -sf --noconfirm || {
            log_warn "Failed to build $pkg (may need dependencies)"
            continue
        }

        # Copy to repo
        cp -f *.pkg.tar.zst "$REPO_DIR/" 2>/dev/null || true

        log_success "Package $pkg built"
    done
}

# Update repository database
update_repo_db() {
    log_info "Updating repository database..."

    cd "$REPO_DIR"

    # Remove old database
    rm -f cvh-linux.db* cvh-linux.files*

    # Create new database
    if ls *.pkg.tar.zst 1> /dev/null 2>&1; then
        repo-add cvh-linux.db.tar.gz *.pkg.tar.zst

        # Replace symlinks with actual files (needed for GitHub raw URLs)
        # repo-add creates symlinks: cvh-linux.db -> cvh-linux.db.tar.gz
        # GitHub raw doesn't follow symlinks, so we need real files
        for link in cvh-linux.db cvh-linux.files; do
            if [[ -L "$link" ]]; then
                target=$(readlink "$link")
                rm "$link"
                cp "$target" "$link"
                log_info "Converted symlink $link to file"
            fi
        done

        log_success "Repository database updated"
    else
        log_warn "No packages found to add to repository"
    fi
}

# Main
main() {
    echo
    echo "╔════════════════════════════════════════════╗"
    echo "║     CVH Linux Package Build Script         ║"
    echo "╚════════════════════════════════════════════╝"
    echo

    check_rust

    # Generate GRUB theme assets
    generate_grub_theme

    # Create PKGBUILDs
    create_fuzzy_pkgbuild
    create_icons_pkgbuild
    create_branding_pkgbuild

    # Build Rust projects first
    build_cvh_fuzzy
    build_cvh_icons

    # Build CVH packages
    build_all_packages

    # Build AUR packages
    build_aur_packages

    # Update repo
    update_repo_db

    echo
    log_success "All packages built successfully!"
    echo
}

main "$@"
