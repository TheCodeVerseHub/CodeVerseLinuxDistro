#!/bin/bash
# Build custom CVH Linux packages
# Creates cvh-fuzzy, cvh-icons, and cvh-branding packages

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
PKGBUILD_DIR="$PROJECT_ROOT/pkgbuild"
SRC_DIR="$PROJECT_ROOT/src"
REPO_DIR="$PROJECT_ROOT/repo/x86_64"

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

    cat > "$PKGBUILD_DIR/cvh-branding/PKGBUILD" <<'EOF'
# Maintainer: CVH Linux Team
pkgname=cvh-branding
pkgver=0.1.0
pkgrel=1
pkgdesc="CVH Linux branding and default configurations"
arch=('any')
url="https://github.com/codeversehub/cvh-linux"
license=('GPL3')
depends=()
source=()

package() {
    # MOTD - welcome message (doesn't conflict with filesystem)
    install -Dm644 /dev/stdin "$pkgdir/etc/motd" <<'MOTDEOF'
Welcome to CVH Linux!

Quick Start:
  - Mod+Return    Open terminal
  - Mod+D         Application launcher
  - Mod+1-9       Switch workspaces
  - Mod+Shift+Q   Close window
  - Mod+Shift+E   Exit Niri

For more info: https://github.com/codeversehub/cvh-linux
MOTDEOF

    # CVH Linux info file (custom location, no conflicts)
    install -Dm644 /dev/stdin "$pkgdir/usr/share/cvh-linux/info" <<'INFOEOF'
NAME="CVH Linux"
PRETTY_NAME="CVH Linux"
ID=cvh
VERSION_ID=0.1
HOME_URL="https://codeversehub.dev"
DOCUMENTATION_URL="https://github.com/codeversehub/cvh-linux"
INFOEOF
}
EOF

    log_success "cvh-branding PKGBUILD created"
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

    # Create PKGBUILDs
    create_fuzzy_pkgbuild
    create_icons_pkgbuild
    create_branding_pkgbuild

    # Build Rust projects first
    build_cvh_fuzzy
    build_cvh_icons

    # Build packages
    build_all_packages

    # Update repo
    update_repo_db

    echo
    log_success "All packages built successfully!"
    echo
}

main "$@"
