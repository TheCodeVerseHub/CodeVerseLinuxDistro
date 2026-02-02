#!/bin/bash
# CVH Linux Installer - Configure Module
# System configuration functions

# Configure the installed system
configure_system() {
    step_header "Configuring System"

    local tasks=(
        "Setting timezone"
        "Generating locales"
        "Setting hostname"
        "Enabling services"
        "Installing bootloader"
        "Creating user account"
        "Setting up shell"
        "Configuring desktop"
    )
    local total=${#tasks[@]}

    # Copy CVH packages and configure repos
    copy_cvh_packages
    create_mirrorlist
    configure_pacman_repos

    # Write the chroot configuration script
    write_chroot_script

    chmod +x /mnt/root/configure.sh

    # Verify script was created
    if [[ ! -f /mnt/root/configure.sh ]]; then
        gum style --foreground 196 "  ✗ Failed to create configuration script!"
        exit 1
    fi

    # Run configuration with progress
    for ((i=0; i<total; i++)); do
        gum spin --spinner dot --title "${tasks[$i]}..." -- sleep 0.5
    done

    gum style --foreground 6 "  ● Running system configuration..."
    arch-chroot /mnt /bin/bash /root/configure.sh
    rm -f /mnt/root/configure.sh

    # Install GRUB bootloader
    install_grub

    gum style --foreground 82 "  ✓ System configured"
}

# Write the chroot configuration script
write_chroot_script() {

    # Write base configuration
    cat > /mnt/root/configure.sh << CONFIGURE_SCRIPT
#!/bin/bash
# Don't use set -e as some commands may fail non-fatally

# Timezone
ln -sf /usr/share/zoneinfo/$TIMEZONE /etc/localtime
hwclock --systohc

# Locale
echo "$LOCALE UTF-8" > /etc/locale.gen
locale-gen >/dev/null 2>&1
echo "LANG=$LOCALE" > /etc/locale.conf

# Keymap
echo "KEYMAP=$KEYMAP" > /etc/vconsole.conf

# Hostname
echo "$HOSTNAME" > /etc/hostname
cat > /etc/hosts << EOF
127.0.0.1   localhost
::1         localhost
127.0.1.1   $HOSTNAME.localdomain $HOSTNAME
EOF

# Configure Ly display manager
mkdir -p /etc/ly
cat > /etc/ly/config.ini << 'LY_EOF'
# CVH Linux Ly Configuration

# Disable the doom-fire background animation
animate = false

# Clean minimal appearance
hide_borders = true

# Run on tty1 (default boot experience)
tty = 1

waylandsessions = /usr/share/wayland-sessions

# Save last session and user
save = true
save_file = /var/cache/ly/save

# Clear password on wrong input
clear_password = true
LY_EOF

mkdir -p /var/cache/ly
chmod 777 /var/cache/ly

# Enable services
systemctl enable NetworkManager >/dev/null 2>&1
systemctl enable systemd-timesyncd >/dev/null 2>&1
systemctl enable seatd >/dev/null 2>&1

# Disable getty on tty1 to prevent conflict with Ly
systemctl disable getty@tty1.service >/dev/null 2>&1

# Enable Ly display manager on tty1
systemctl enable ly@tty1.service >/dev/null 2>&1

# Install CVH custom packages from local cache
echo "Installing CVH custom packages..."
if ls /var/cache/pacman/cvh-packages/*.pkg.tar.zst >/dev/null 2>&1; then
    pacman -U --noconfirm /var/cache/pacman/cvh-packages/*.pkg.tar.zst >/dev/null 2>&1 || true

    # Verify installation
    echo "Verifying CVH packages:"
    for pkg in cvh-fuzzy cvh-icons cvh-branding; do
        if pacman -Q \$pkg >/dev/null 2>&1; then
            echo "  ✓ \$pkg installed"
        else
            echo "  ✗ \$pkg not installed (optional)"
        fi
    done
else
    echo "  ⚠ CVH packages not found, skipping"
fi

# Create os-release for proper branding (GRUB uses this)
cat > /etc/os-release << EOF
NAME="CVH Linux"
PRETTY_NAME="CVH Linux"
ID=cvh
ID_LIKE=arch
BUILD_ID=rolling
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://cvhlinux.org"
DOCUMENTATION_URL="https://wiki.cvhlinux.org"
LOGO=cvh-logo
EOF

# Set GRUB distributor name
sed -i 's/^GRUB_DISTRIBUTOR=.*/GRUB_DISTRIBUTOR="CVH Linux"/' /etc/default/grub 2>/dev/null || \
    echo 'GRUB_DISTRIBUTOR="CVH Linux"' >> /etc/default/grub

# Install CVH Nordic GRUB theme
if [[ -d /usr/share/cvh-linux/grub-theme/cvh-nordic ]]; then
    echo "Installing GRUB theme..."
    mkdir -p /boot/grub/themes
    cp -r /usr/share/cvh-linux/grub-theme/cvh-nordic /boot/grub/themes/

    # Configure GRUB to use the theme
    sed -i 's|^#*GRUB_THEME=.*|GRUB_THEME="/boot/grub/themes/cvh-nordic/theme.txt"|' /etc/default/grub 2>/dev/null
    grep -q '^GRUB_THEME=' /etc/default/grub || \
        echo 'GRUB_THEME="/boot/grub/themes/cvh-nordic/theme.txt"' >> /etc/default/grub
    echo "  ✓ GRUB theme installed"
else
    echo "  ⚠ GRUB theme not found, skipping"
fi

# Ensure zsh is in /etc/shells
echo "/usr/bin/zsh" >> /etc/shells
echo "/bin/zsh" >> /etc/shells

# Create seat group if it doesn't exist (for seatd)
getent group seat >/dev/null || groupadd seat

# Create user
useradd -m -G wheel,audio,video,input,seat -s /usr/bin/zsh $USERNAME

# Force set shell using usermod (more reliable than chsh)
usermod -s /usr/bin/zsh $USERNAME

# Also set root's shell to zsh
usermod -s /usr/bin/zsh root

# Verify shell was set
echo "User shell set to: \$(getent passwd $USERNAME | cut -d: -f7)"
echo "Root shell set to: \$(getent passwd root | cut -d: -f7)"

# Configure sudo
echo "%wheel ALL=(ALL:ALL) ALL" > /etc/sudoers.d/wheel
chmod 440 /etc/sudoers.d/wheel

# Set up Oh My Zsh
su - $USERNAME -c 'git clone --depth=1 https://github.com/ohmyzsh/ohmyzsh.git ~/.oh-my-zsh 2>/dev/null' || true
su - $USERNAME -c 'cp ~/.oh-my-zsh/templates/zshrc.zsh-template ~/.zshrc 2>/dev/null' || true

# Set up niri compositor config
su - $USERNAME -c 'mkdir -p ~/.config/niri'
# Copy niri config from /etc/skel (synced during ISO build)
if [[ -d /etc/skel/.config/niri ]]; then
    cp -r /etc/skel/.config/niri/* /home/$USERNAME/.config/niri/
fi
chown -R $USERNAME:$USERNAME /home/$USERNAME/.config/niri

# Create Wayland session file for Ly display manager
mkdir -p /usr/share/wayland-sessions
cat > /usr/share/wayland-sessions/niri.desktop << 'SESSION_EOF'
[Desktop Entry]
Name=Niri
Comment=Scrollable-tiling Wayland compositor
Exec=niri-session
Type=Application
SESSION_EOF

# Create .zshrc
cat > /home/$USERNAME/.zshrc << 'ZSHRC_EOF'
export ZSH="\$HOME/.oh-my-zsh"
ZSH_THEME="robbyrussell"
plugins=(git sudo history)
[[ -f \$ZSH/oh-my-zsh.sh ]] && source \$ZSH/oh-my-zsh.sh

export EDITOR="nano"
export QT_QPA_PLATFORM="wayland"
export MOZ_ENABLE_WAYLAND="1"

# ZSH History Configuration
export HISTFILE="\$HOME/.zsh_history"
export HISTSIZE=10000
export SAVEHIST=10000
setopt APPEND_HISTORY
setopt SHARE_HISTORY
setopt HIST_IGNORE_DUPS
setopt HIST_IGNORE_ALL_DUPS
setopt HIST_REDUCE_BLANKS
setopt HIST_SAVE_NO_DUPS
setopt INC_APPEND_HISTORY

alias ls='ls --color=auto'
alias ll='ls -la'

# Fallback compositor auto-start on tty2+ (Ly runs on tty1)
# This triggers if user switches to another TTY or Ly is not running
if [[ -z "\$WAYLAND_DISPLAY" ]] && [[ "\$XDG_VTNR" -ne 1 ]]; then
    exec COMPOSITOR_SESSION
fi
ZSHRC_EOF

# Create initial history file with proper permissions
su - $USERNAME -c 'touch ~/.zsh_history'
su - $USERNAME -c 'chmod 600 ~/.zsh_history'

# Set niri session in zshrc
sed -i 's/COMPOSITOR_SESSION/niri-session/g' /home/$USERNAME/.zshrc

chown $USERNAME:$USERNAME /home/$USERNAME/.zshrc

# Create fastfetch config with custom ASCII art support
su - $USERNAME -c 'mkdir -p ~/.config/fastfetch'
cat > /home/$USERNAME/.config/fastfetch/config.jsonc << 'FASTFETCH_EOF'
{
    "\$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
    "logo": {
        "type": "file",
        "source": "~/.config/fastfetch/ascii_art.txt",
        "color": {
            "1": "cyan",
            "2": "blue",
            "3": "white"
        }
    },
    "display": {
        "separator": " -> ",
        "color": {
            "separator": "blue"
        }
    },
    "modules": [
        {
            "type": "title",
            "format": "{user-name}@{host-name}"
        },
        {
            "type": "separator",
            "string": "─────────────────────────────"
        },
        {
            "type": "os",
            "key": "OS",
            "format": "{3}"
        },
        {
            "type": "kernel",
            "key": "Kernel"
        },
        {
            "type": "packages",
            "key": "Packages"
        },
        {
            "type": "shell",
            "key": "Shell"
        },
        {
            "type": "display",
            "key": "Display (WM)"
        },
        {
            "type": "terminal",
            "key": "Terminal"
        },
        {
            "type": "cpu",
            "key": "CPU"
        },
        {
            "type": "gpu",
            "key": "GPU"
        },
        {
            "type": "memory",
            "key": "Memory"
        },
        {
            "type": "uptime",
            "key": "Uptime"
        },
        {
            "type": "colors",
            "symbol": "circle"
        }
    ]
}
FASTFETCH_EOF

# Create custom ASCII art template
cat > /home/$USERNAME/.config/fastfetch/ascii_art.txt << 'ASCII_EOF'
     ██████╗██╗   ██╗██╗  ██╗    ██╗     ██╗███╗   ██╗██╗   ██╗██╗  ██╗
    ██╔════╝██║   ██║██║  ██║    ██║     ██║████╗  ██║██║   ██║╚██╗██╔╝
    ██║     ██║   ██║███████║    ██║     ██║██╔██╗ ██║██║   ██║ ╚███╔╝
    ██║     ╚██╗ ██╔╝██╔══██║    ██║     ██║██║╚██╗██║██║   ██║ ██╔██╗
    ╚██████╗ ╚████╔╝ ██║  ██║    ███████╗██║██║ ╚████║╚██████╔╝██╔╝ ██╗
     ╚═════╝  ╚═══╝  ╚═╝  ╚═╝    ╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝

                          CodeVerse Hub Linux
ASCII_EOF

# Create instructions file for custom ASCII art
cat > /home/$USERNAME/.config/fastfetch/README.md << 'README_EOF'
# Fastfetch Custom ASCII Art

## Using Your Own ASCII Art

To use custom ASCII art with fastfetch:

1. Edit \`ascii_art.txt\` with your custom ASCII art
2. Update \`config.jsonc\` to use your custom art:

Replace the \`logo\` section with:
\`\`\`json
"logo": {
    "type": "file",
    "source": "~/.config/fastfetch/ascii_art.txt",
    "color": {
        "1": "cyan",
        "2": "blue",
        "3": "white"
    }
}
\`\`\`

## Color Options

Available colors: black, red, green, yellow, blue, magenta, cyan, white

## Built-in Logos

To use a built-in logo instead, change \`source\` to:
- \`arch_small\` - Small Arch logo (default)
- \`arch\` - Full Arch logo
- \`linux\` - Generic Linux logo
- \`none\` - No logo

## Run Fastfetch

Simply type:
\`\`\`bash
fastfetch
\`\`\`

Or add it to your \`.zshrc\` to run on terminal startup.
README_EOF

chown -R $USERNAME:$USERNAME /home/$USERNAME/.config/fastfetch

chown $USERNAME:$USERNAME /home/$USERNAME/.zshrc
CONFIGURE_SCRIPT
}

# Install GRUB bootloader
install_grub() {
    gum style --foreground 6 "  ● Installing GRUB bootloader..."
    if [[ "$BOOT_MODE" == "uefi" ]]; then
        arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=CVH
    else
        arch-chroot /mnt grub-install --target=i386-pc "$DISK"
    fi

    gum style --foreground 6 "  ● Generating GRUB config..."
    arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg
}
