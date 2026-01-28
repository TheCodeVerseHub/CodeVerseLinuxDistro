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
    local compositor_session="niri-session"
    [[ "$COMPOSITOR" == "hyprland" ]] && compositor_session="Hyprland"

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

animation = 0
hide_borders = 0

# Run on tty1 (default boot experience)
tty = 1

waylandsessions = /usr/share/wayland-sessions

# Save last session and user
save = 1
save_file = /var/cache/ly/save
LY_EOF

mkdir -p /var/cache/ly
chmod 755 /var/cache/ly

# Enable services
systemctl enable NetworkManager >/dev/null 2>&1
systemctl enable systemd-timesyncd >/dev/null 2>&1
systemctl enable seatd >/dev/null 2>&1
systemctl enable ly.service >/dev/null 2>&1

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

# Create compositor config based on selection
if [[ "$COMPOSITOR" == "niri" ]]; then
    su - $USERNAME -c 'mkdir -p ~/.config/niri'
    cat > /home/$USERNAME/.config/niri/config.kdl << 'NIRI_EOF'
input {
    keyboard {
        xkb {
            layout "us"
        }
    }
    touchpad {
        tap
        natural-scroll
    }
}

layout {
    gaps 8

    focus-ring {
        width 2
        active-color "#88c0d0"
    }
}

spawn-at-startup "mako"

binds {
    Mod+Return { spawn "foot"; }
    Mod+D { spawn "fuzzel"; }
    Mod+Shift+Q { close-window; }
    Mod+Shift+E { quit; }

    Mod+H { focus-column-left; }
    Mod+J { focus-window-down; }
    Mod+K { focus-window-up; }
    Mod+L { focus-column-right; }

    Mod+1 { focus-workspace 1; }
    Mod+2 { focus-workspace 2; }
    Mod+3 { focus-workspace 3; }
    Mod+4 { focus-workspace 4; }
    Mod+5 { focus-workspace 5; }
    Mod+6 { focus-workspace 6; }
    Mod+7 { focus-workspace 7; }
    Mod+8 { focus-workspace 8; }
    Mod+9 { focus-workspace 9; }

    Print { spawn "sh" "-c" "grim -g \\\"\$(slurp)\\\" ~/Pictures/screenshot-\$(date +%Y%m%d-%H%M%S).png"; }
}
NIRI_EOF
    chown -R $USERNAME:$USERNAME /home/$USERNAME/.config/niri

elif [[ "$COMPOSITOR" == "hyprland" ]]; then
    su - $USERNAME -c 'mkdir -p ~/.config/hypr'
    cat > /home/$USERNAME/.config/hypr/hyprland.conf << 'HYPR_EOF'
# CVH Linux Hyprland Configuration

monitor=,preferred,auto,auto

\$terminal = foot
\$menu = cvh-fuzzy --mode apps

env = QT_QPA_PLATFORM,wayland
env = MOZ_ENABLE_WAYLAND,1
env = XCURSOR_THEME,Adwaita
env = XCURSOR_SIZE,24

input {
    kb_layout = us
    repeat_delay = 300
    repeat_rate = 50
    touchpad {
        natural_scroll = true
        tap-to-click = true
    }
}

general {
    gaps_in = 8
    gaps_out = 8
    border_size = 2
    col.active_border = rgba(88c0d0ff)
    col.inactive_border = rgba(4c566aff)
    layout = dwindle
}

decoration {
    rounding = 0
    blur { enabled = false }
    drop_shadow = false
}

animations {
    enabled = true
    bezier = easeOut, 0.16, 1, 0.3, 1
    animation = windows, 1, 3, easeOut, slide
    animation = workspaces, 1, 4, easeOut, slide
}

exec-once = cvh-icons
exec-once = mako
exec-once = /usr/lib/polkit-gnome/polkit-gnome-authentication-agent-1

\$mainMod = SUPER

bind = \$mainMod, RETURN, exec, \$terminal
bind = \$mainMod, D, exec, \$menu
bind = \$mainMod SHIFT, Q, killactive
bind = \$mainMod SHIFT, E, exit

# Focus (vim-style and arrows)
bind = \$mainMod, H, movefocus, l
bind = \$mainMod, L, movefocus, r
bind = \$mainMod, K, movefocus, u
bind = \$mainMod, J, movefocus, d
bind = \$mainMod, LEFT, movefocus, l
bind = \$mainMod, RIGHT, movefocus, r
bind = \$mainMod, UP, movefocus, u
bind = \$mainMod, DOWN, movefocus, d

# Move windows
bind = \$mainMod SHIFT, H, movewindow, l
bind = \$mainMod SHIFT, L, movewindow, r
bind = \$mainMod SHIFT, K, movewindow, u
bind = \$mainMod SHIFT, J, movewindow, d

# Workspaces
bind = \$mainMod, 1, workspace, 1
bind = \$mainMod, 2, workspace, 2
bind = \$mainMod, 3, workspace, 3
bind = \$mainMod, 4, workspace, 4
bind = \$mainMod, 5, workspace, 5
bind = \$mainMod, 6, workspace, 6
bind = \$mainMod, 7, workspace, 7
bind = \$mainMod, 8, workspace, 8
bind = \$mainMod, 9, workspace, 9

bind = \$mainMod SHIFT, 1, movetoworkspace, 1
bind = \$mainMod SHIFT, 2, movetoworkspace, 2
bind = \$mainMod SHIFT, 3, movetoworkspace, 3
bind = \$mainMod SHIFT, 4, movetoworkspace, 4
bind = \$mainMod SHIFT, 5, movetoworkspace, 5
bind = \$mainMod SHIFT, 6, movetoworkspace, 6
bind = \$mainMod SHIFT, 7, movetoworkspace, 7
bind = \$mainMod SHIFT, 8, movetoworkspace, 8
bind = \$mainMod SHIFT, 9, movetoworkspace, 9

# Screenshots
bind = , PRINT, exec, grim -g "\\\$(slurp)" ~/Pictures/Screenshots/screenshot-\\\$(date +%Y-%m-%d-%H-%M-%S).png
bind = \$mainMod, PRINT, exec, grim ~/Pictures/Screenshots/screenshot-\\\$(date +%Y-%m-%d-%H-%M-%S).png

# Audio
bindl = , XF86AudioRaiseVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%+
bindl = , XF86AudioLowerVolume, exec, wpctl set-volume @DEFAULT_AUDIO_SINK@ 5%-
bindl = , XF86AudioMute, exec, wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle

# Brightness
bind = , XF86MonBrightnessUp, exec, brightnessctl set 5%+
bind = , XF86MonBrightnessDown, exec, brightnessctl set 5%-

bindm = \$mainMod, mouse:272, movewindow
bindm = \$mainMod, mouse:273, resizewindow
HYPR_EOF
    chown -R $USERNAME:$USERNAME /home/$USERNAME/.config/hypr
fi

# Create Wayland session file for Ly display manager
mkdir -p /usr/share/wayland-sessions

if [[ "$COMPOSITOR" == "niri" ]]; then
    cat > /usr/share/wayland-sessions/niri.desktop << 'SESSION_EOF'
[Desktop Entry]
Name=Niri
Comment=Scrollable-tiling Wayland compositor
Exec=niri-session
Type=Application
SESSION_EOF

elif [[ "$COMPOSITOR" == "hyprland" ]]; then
    cat > /usr/share/wayland-sessions/hyprland.desktop << 'SESSION_EOF'
[Desktop Entry]
Name=Hyprland
Comment=Dynamic tiling Wayland compositor
Exec=Hyprland
Type=Application
SESSION_EOF
fi

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

# Replace compositor session based on selection
if [[ "$COMPOSITOR" == "niri" ]]; then
    sed -i 's/COMPOSITOR_SESSION/niri-session/g' /home/$USERNAME/.zshrc
elif [[ "$COMPOSITOR" == "hyprland" ]]; then
    sed -i 's/COMPOSITOR_SESSION/Hyprland/g' /home/$USERNAME/.zshrc
fi

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
