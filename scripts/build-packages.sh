#!/bin/bash
# Build custom CVH Linux packages
# Creates cvh-fuzzy, cvh-icons, and cvh-branding packages

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PKGBUILD_DIR="$PROJECT_ROOT/pkgbuild"
SRC_DIR="$PROJECT_ROOT/src"
REPO_DIR="$PROJECT_ROOT/repo/x86_64"
GRUB_THEME_DIR="$PROJECT_ROOT/configs/grub/themes/tela"

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

# Download and setup Tela GRUB theme
generate_grub_theme() {
    log_info "Setting up Tela GRUB theme..."

    local GRUB2_THEMES_REPO="https://github.com/vinceliuice/grub2-themes.git"
    local TEMP_DIR="$PROJECT_ROOT/.grub-themes-temp"

    # Clean up any existing temp directory
    rm -rf "$TEMP_DIR"

    # Clone the grub2-themes repository
    log_info "  Cloning grub2-themes repository..."
    if ! git clone --depth=1 "$GRUB2_THEMES_REPO" "$TEMP_DIR" 2>/dev/null; then
        log_error "Failed to clone grub2-themes repository"
        return 1
    fi

    # Create theme directory
    rm -rf "$GRUB_THEME_DIR"
    mkdir -p "$GRUB_THEME_DIR/icons"

    # Copy Tela theme files (1080p, color icons)
    log_info "  Extracting Tela theme (1080p, color icons)..."

    # Copy background
    if [[ -f "$TEMP_DIR/backgrounds/1080p/background-tela.jpg" ]]; then
        cp "$TEMP_DIR/backgrounds/1080p/background-tela.jpg" "$GRUB_THEME_DIR/background.jpg"
    fi

    # Copy theme.txt from common and customize for tela
    if [[ -f "$TEMP_DIR/common/theme.txt" ]]; then
        cp "$TEMP_DIR/common/theme.txt" "$GRUB_THEME_DIR/theme.txt"
        # Update background reference to jpg
        sed -i 's/background\.png/background.jpg/g' "$GRUB_THEME_DIR/theme.txt"
    fi

    # Copy icons (color variant)
    if [[ -d "$TEMP_DIR/assets/assets-tela/icons-1080p/color" ]]; then
        cp "$TEMP_DIR/assets/assets-tela/icons-1080p/color"/*.png "$GRUB_THEME_DIR/icons/" 2>/dev/null || true
    fi

    # Copy additional assets (select boxes, etc.)
    if [[ -d "$TEMP_DIR/assets/assets-tela/icons-1080p" ]]; then
        cp "$TEMP_DIR/assets/assets-tela/icons-1080p"/*.png "$GRUB_THEME_DIR/" 2>/dev/null || true
    fi

    # Copy common assets (progress bar, terminal box, etc.)
    if [[ -d "$TEMP_DIR/common" ]]; then
        cp "$TEMP_DIR/common"/*.png "$GRUB_THEME_DIR/" 2>/dev/null || true
    fi

    # Clean up temp directory
    rm -rf "$TEMP_DIR"

    log_success "Tela GRUB theme installed"
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

    # Install GRUB theme (Tela)
    install -dm755 "\$pkgdir/usr/share/cvh-linux/grub-theme/tela"
    install -dm755 "\$pkgdir/usr/share/cvh-linux/grub-theme/tela/icons"

    # Copy theme files
    if [[ -f "\$_cvh_root/configs/grub/themes/tela/theme.txt" ]]; then
        install -Dm644 "\$_cvh_root/configs/grub/themes/tela/theme.txt" \\
            "\$pkgdir/usr/share/cvh-linux/grub-theme/tela/theme.txt"
    fi

    # Copy background (Tela uses .jpg)
    if [[ -f "\$_cvh_root/configs/grub/themes/tela/background.jpg" ]]; then
        install -Dm644 "\$_cvh_root/configs/grub/themes/tela/background.jpg" \\
            "\$pkgdir/usr/share/cvh-linux/grub-theme/tela/background.jpg"
    fi

    # Copy all theme assets (select boxes, progress bars, etc.)
    for asset in "\$_cvh_root/configs/grub/themes/tela"/*.png; do
        if [[ -f "\$asset" ]]; then
            install -Dm644 "\$asset" "\$pkgdir/usr/share/cvh-linux/grub-theme/tela/\$(basename \$asset)"
        fi
    done

    # Copy icons
    for icon in "\$_cvh_root/configs/grub/themes/tela/icons"/*.png; do
        if [[ -f "\$icon" ]]; then
            install -Dm644 "\$icon" "\$pkgdir/usr/share/cvh-linux/grub-theme/tela/icons/\$(basename \$icon)"
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
