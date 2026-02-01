#!/bin/bash
# Hyprland Screen Sharing Setup Script
# This script installs and configures screen sharing for Hyprland

set -e

echo "=== Hyprland Screen Sharing Setup ==="
echo ""

# Check if running on Arch-based system
if ! command -v pacman &> /dev/null; then
    echo "Error: This script is designed for Arch Linux based distributions"
    exit 1
fi

# Check if running as root (should NOT be)
if [ "$EUID" -eq 0 ]; then
    echo "Error: Please do not run this script as root"
    exit 1
fi

echo "Step 1: Installing required packages..."
sudo pacman -S --needed --noconfirm xdg-desktop-portal-hyprland pipewire wireplumber xdg-desktop-portal 2>/dev/null || {
    echo "Some packages may already be installed"
}

echo ""
echo "Step 2: Enabling services..."
# Enable and start user services
systemctl --user enable pipewire.service
systemctl --user enable wireplumber.service
systemctl --user enable pipewire.socket
systemctl --user start pipewire.socket
systemctl --user start pipewire.service
systemctl --user start wireplumber.service

echo ""
echo "Step 3: Creating portal configuration..."

# Create portal configuration directory
mkdir -p ~/.config/xdg-desktop-portal

# Create Hyprland portal config
cat > ~/.config/xdg-desktop-portal/hyprland-portals.conf << 'EOF'
[preferred]
default=hyprland
org.freedesktop.impl.portal.Screenshot=hyprland
org.freedesktop.impl.portal.ScreenCast=hyprland
org.freedesktop.impl.portal.RemoteDesktop=hyprland
EOF

echo ""
echo "Step 4: Checking environment variables..."

# Check if XDG_CURRENT_DESKTOP is set correctly
if [ -z "$XDG_CURRENT_DESKTOP" ]; then
    echo "WARNING: XDG_CURRENT_DESKTOP is not set!"
    echo "Add this to your Hyprland config or ~/.bashrc/.zshrc:"
    echo "  export XDG_CURRENT_DESKTOP=Hyprland"
fi

# Check if XDG_SESSION_TYPE is set correctly
if [ "$XDG_SESSION_TYPE" != "wayland" ]; then
    echo "WARNING: XDG_SESSION_TYPE is not set to 'wayland'!"
    echo "Add this to your Hyprland config or ~/.bashrc/.zshrc:"
    echo "  export XDG_SESSION_TYPE=wayland"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To verify screen sharing works:"
echo "1. Restart your Hyprland session (log out and log back in)"
echo "2. Open a browser and go to: https://mozilla.github.io/webrtc-landing/gum_test.html"
echo "3. Try sharing your screen"
echo ""
echo "Troubleshooting:"
echo "- If Chrome/Discord can't share Wayland windows, use --enable-features=WaylandWindowDecorations --ozone-platform-hint=auto"
echo "- Check 'systemctl --user status xdg-desktop-portal-hyprland' if issues occur"
echo "- Ensure your monitor bit depth matches in Hyprland config"
