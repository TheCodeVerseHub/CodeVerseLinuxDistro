#!/bin/bash
# CodeVerse Linux Installer
# Run from live environment to install CodeVerse Linux to disk

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
MAGENTA='\033[0;35m'
BOLD='\033[1m'
DIM='\033[2m'
NC='\033[0m'

# Progress tracking
TOTAL_STEPS=10
CURRENT_STEP=0

# Global variables
DISK=""
BOOT_MODE=""
EFI_PART=""
ROOT_PART=""
USERNAME="cvh"
HOSTNAME="cvh-linux"
TIMEZONE="Asia/Jerusalem"
LOCALE="en_US.UTF-8"
KEYMAP="us"
COMPOSITOR=""  # Will be "niri" or "hyprland"

# Logging functions
log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Progress bar function
# Usage: progress_bar <current> <total> <label>
progress_bar() {
    local current=$1
    local total=$2
    local label=${3:-"Progress"}
    local width=40
    local percent=$((current * 100 / total))
    local filled=$((current * width / total))
    local empty=$((width - filled))

    # Build the bar
    local bar=""
    for ((i=0; i<filled; i++)); do bar+="█"; done
    for ((i=0; i<empty; i++)); do bar+="░"; done

    printf "\r${CYAN}%s${NC} [${GREEN}%s${NC}] ${BOLD}%3d%%${NC}" "$label" "$bar" "$percent"
}

# Spinner function for background tasks
# Usage: run_with_spinner "message" command args...
run_with_spinner() {
    local message="$1"
    shift
    local spin_chars='⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏'
    local pid

    # Run command in background
    "$@" &>/dev/null &
    pid=$!

    # Show spinner while command runs
    local i=0
    while kill -0 $pid 2>/dev/null; do
        local char="${spin_chars:i++%${#spin_chars}:1}"
        printf "\r${CYAN}%s${NC} %s" "$char" "$message"
        sleep 0.1
    done

    # Check exit status
    wait $pid
    local status=$?

    if [[ $status -eq 0 ]]; then
        printf "\r${GREEN}✓${NC} %s\n" "$message"
    else
        printf "\r${RED}✗${NC} %s\n" "$message"
        return $status
    fi
}

# Step header with progress
step_header() {
    ((CURRENT_STEP++))
    echo
    echo -e "${BOLD}${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${BOLD}  Step ${CURRENT_STEP}/${TOTAL_STEPS}: $1${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo
}

# Overall progress indicator
show_overall_progress() {
    local width=60
    local filled=$((CURRENT_STEP * width / TOTAL_STEPS))
    local empty=$((width - filled))
    local percent=$((CURRENT_STEP * 100 / TOTAL_STEPS))

    echo
    echo -ne "${DIM}Overall Progress: ${NC}"
    echo -ne "${GREEN}"
    for ((i=0; i<filled; i++)); do echo -n "▓"; done
    echo -ne "${NC}${DIM}"
    for ((i=0; i<empty; i++)); do echo -n "░"; done
    echo -e "${NC} ${BOLD}${percent}%${NC}"
}

# Check if running as root
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log_error "This installer must be run as root"
        exit 1
    fi
}

# Display welcome banner
show_welcome() {
    clear
    echo
    echo -e "${BOLD}${CYAN}"
    cat << 'EOF'
   ██████╗██╗   ██╗██╗  ██╗    ██╗     ██╗███╗   ██╗██╗   ██╗██╗  ██╗
  ██╔════╝██║   ██║██║  ██║    ██║     ██║████╗  ██║██║   ██║╚██╗██╔╝
  ██║     ██║   ██║███████║    ██║     ██║██╔██╗ ██║██║   ██║ ╚███╔╝
  ██║     ╚██╗ ██╔╝██╔══██║    ██║     ██║██║╚██╗██║██║   ██║ ██╔██╗
  ╚██████╗ ╚████╔╝ ██║  ██║    ███████╗██║██║ ╚████║╚██████╔╝██╔╝ ██╗
   ╚═════╝  ╚═══╝  ╚═╝  ╚═╝    ╚══════╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═╝  ╚═╝
EOF
    echo -e "${NC}"
    echo -e "${DIM}                    CodeVerse Hub Linux Distribution${NC}"
    echo
    echo -e "  ${BOLD}Features:${NC}"
    echo -e "    ${GREEN}●${NC} Niri or Hyprland Wayland compositor"
    echo -e "    ${GREEN}●${NC} Ly display manager"
    echo -e "    ${GREEN}●${NC} Zsh + Oh My Zsh"
    echo -e "    ${GREEN}●${NC} Custom fuzzy finder & icon system"
    echo -e "    ${GREEN}●${NC} Minimal & lightweight"
    echo
    echo -e "  ${YELLOW}⚠${NC}  ${BOLD}WARNING:${NC} This will ERASE all data on the selected disk!"
    echo
    read -r -p "  Press Enter to begin installation or Ctrl+C to cancel..." _ || true
}

# Detect boot mode (UEFI or BIOS)
detect_boot_mode() {
    step_header "Detecting System"

    echo -n "  Checking boot mode... "
    if [[ -d /sys/firmware/efi/efivars ]]; then
        BOOT_MODE="uefi"
        echo -e "${GREEN}UEFI${NC}"
    else
        BOOT_MODE="bios"
        echo -e "${YELLOW}BIOS/Legacy${NC}"
    fi
}

# Select keyboard layout
select_keyboard() {
    step_header "Keyboard Layout"

    echo "  Available layouts:"
    echo -e "    ${BOLD}1)${NC} us - US English ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} uk - UK English"
    echo -e "    ${BOLD}3)${NC} de - German"
    echo -e "    ${BOLD}4)${NC} fr - French"
    echo -e "    ${BOLD}5)${NC} es - Spanish"
    echo -e "    ${BOLD}6)${NC} il - Hebrew"
    echo -e "    ${BOLD}7)${NC} Other"
    echo

    read -r -p "  Select layout [1]: " kb_choice
    kb_choice=${kb_choice:-1}

    case $kb_choice in
        1) KEYMAP="us" ;;
        2) KEYMAP="uk" ;;
        3) KEYMAP="de" ;;
        4) KEYMAP="fr" ;;
        5) KEYMAP="es" ;;
        6) KEYMAP="il" ;;
        7) read -r -p "  Enter keymap name: " KEYMAP ;;
        *) KEYMAP="us" ;;
    esac

    loadkeys "$KEYMAP" 2>/dev/null || true
    echo -e "\n  ${GREEN}✓${NC} Keyboard: ${BOLD}$KEYMAP${NC}"
}

# Select timezone
select_timezone() {
    step_header "Timezone"

    echo "  Common timezones:"
    echo -e "    ${BOLD}1)${NC} Asia/Jerusalem ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} UTC"
    echo -e "    ${BOLD}3)${NC} America/New_York"
    echo -e "    ${BOLD}4)${NC} America/Los_Angeles"
    echo -e "    ${BOLD}5)${NC} Europe/London"
    echo -e "    ${BOLD}6)${NC} Europe/Berlin"
    echo -e "    ${BOLD}7)${NC} Other"
    echo

    read -r -p "  Select timezone [1]: " tz_choice
    tz_choice=${tz_choice:-1}

    case $tz_choice in
        1) TIMEZONE="Asia/Jerusalem" ;;
        2) TIMEZONE="UTC" ;;
        3) TIMEZONE="America/New_York" ;;
        4) TIMEZONE="America/Los_Angeles" ;;
        5) TIMEZONE="Europe/London" ;;
        6) TIMEZONE="Europe/Berlin" ;;
        7) read -r -p "  Enter timezone (Region/City): " TIMEZONE ;;
        *) TIMEZONE="Asia/Jerusalem" ;;
    esac

    echo -e "\n  ${GREEN}✓${NC} Timezone: ${BOLD}$TIMEZONE${NC}"
}

# Select compositor
select_compositor() {
    step_header "Compositor Selection"

    echo "  Available Wayland compositors:"
    echo -e "    ${BOLD}1)${NC} Niri - Scrollable-tiling compositor ${DIM}[default]${NC}"
    echo -e "    ${BOLD}2)${NC} Hyprland - Dynamic tiling compositor"
    echo

    read -r -p "  Select compositor [1]: " comp_choice
    comp_choice=${comp_choice:-1}

    case $comp_choice in
        1) COMPOSITOR="niri" ;;
        2) COMPOSITOR="hyprland" ;;
        *) COMPOSITOR="niri" ;;
    esac

    echo -e "\n  ${GREEN}✓${NC} Compositor: ${BOLD}$COMPOSITOR${NC}"
}

# Select disk for installation
select_disk() {
    step_header "Disk Selection"

    echo "  Available disks:"
    echo
    # Filter and display disks with nice formatting
    local i=1
    while IFS= read -r line; do
        local name=$(echo "$line" | awk '{print $1}')
        local size=$(echo "$line" | awk '{print $2}')
        local model=$(echo "$line" | awk '{$1=$2=""; print $0}' | xargs)
        printf "    ${BOLD}%d)${NC} /dev/%-8s ${CYAN}%8s${NC}  %s\n" "$i" "$name" "$size" "$model"
        ((i++))
    done < <(lsblk -dno NAME,SIZE,MODEL | grep -vE "^(loop|sr|rom|fd|zram)")
    echo

    # Get list of disks
    local disks=($(lsblk -dno NAME | grep -vE "^(loop|sr|rom|fd|zram)"))

    if [[ ${#disks[@]} -eq 0 ]]; then
        log_error "No suitable disks found!"
        exit 1
    fi

    read -r -p "  Enter disk number: " disk_num

    if [[ ! "$disk_num" =~ ^[0-9]+$ ]] || [[ $disk_num -lt 1 ]] || [[ $disk_num -gt ${#disks[@]} ]]; then
        log_error "Invalid selection!"
        exit 1
    fi

    DISK="/dev/${disks[$((disk_num-1))]}"

    echo
    echo -e "  ${YELLOW}⚠${NC}  Selected: ${BOLD}$DISK${NC}"
    echo -e "  ${RED}    ALL DATA WILL BE DESTROYED!${NC}"
    echo
    read -r -p "  Type 'yes' to confirm: " confirm
    if [[ "$confirm" != "yes" ]]; then
        log_error "Installation cancelled"
        exit 1
    fi

    echo -e "\n  ${GREEN}✓${NC} Disk: ${BOLD}$DISK${NC}"
}

# Set hostname
set_hostname() {
    step_header "System Configuration"

    read -r -p "  Enter hostname [cvh-linux]: " input_hostname
    HOSTNAME=${input_hostname:-cvh-linux}

    if [[ ! "$HOSTNAME" =~ ^[a-zA-Z0-9]([a-zA-Z0-9-]*[a-zA-Z0-9])?$ ]]; then
        log_warn "Invalid hostname, using: cvh-linux"
        HOSTNAME="cvh-linux"
    fi

    echo -e "  ${GREEN}✓${NC} Hostname: ${BOLD}$HOSTNAME${NC}"
}

# Create user account
create_user_config() {
    echo
    read -r -p "  Enter username [cvh]: " input_username
    USERNAME=${input_username:-cvh}

    if [[ ! "$USERNAME" =~ ^[a-z_][a-z0-9_-]*$ ]]; then
        log_warn "Invalid username, using: cvh"
        USERNAME="cvh"
    fi

    echo -e "  ${GREEN}✓${NC} Username: ${BOLD}$USERNAME${NC}"
}

# Partition the disk
partition_disk() {
    step_header "Partitioning Disk"

    echo "  Preparing disk..."

    # Wipe existing partition table
    wipefs -af "$DISK" >/dev/null 2>&1 || true
    sgdisk -Z "$DISK" >/dev/null 2>&1 || true

    if [[ "$BOOT_MODE" == "uefi" ]]; then
        echo -e "  ${BLUE}●${NC} Creating GPT partition table (UEFI)"

        parted -s "$DISK" mklabel gpt
        progress_bar 1 5 "  Partitioning"

        parted -s "$DISK" mkpart primary fat32 1MiB 513MiB
        parted -s "$DISK" set 1 esp on
        progress_bar 2 5 "  Partitioning"

        parted -s "$DISK" mkpart primary ext4 513MiB 100%
        progress_bar 3 5 "  Partitioning"

        if [[ "$DISK" == *"nvme"* ]] || [[ "$DISK" == *"mmcblk"* ]]; then
            EFI_PART="${DISK}p1"
            ROOT_PART="${DISK}p2"
        else
            EFI_PART="${DISK}1"
            ROOT_PART="${DISK}2"
        fi

        sleep 1  # Wait for kernel to recognize partitions

        echo -e "\n  ${BLUE}●${NC} Formatting EFI partition (FAT32)"
        mkfs.fat -F32 "$EFI_PART" >/dev/null 2>&1
        progress_bar 4 5 "  Formatting "

        echo -e "\n  ${BLUE}●${NC} Formatting root partition (ext4)"
        mkfs.ext4 -F "$ROOT_PART" >/dev/null 2>&1
        progress_bar 5 5 "  Formatting "

        echo -e "\n\n  ${BLUE}●${NC} Mounting partitions"
        mount "$ROOT_PART" /mnt
        mkdir -p /mnt/boot/efi
        mount "$EFI_PART" /mnt/boot/efi

    else
        echo -e "  ${BLUE}●${NC} Creating MBR partition table (BIOS)"

        parted -s "$DISK" mklabel msdos
        progress_bar 1 4 "  Partitioning"

        parted -s "$DISK" mkpart primary ext4 1MiB 100%
        parted -s "$DISK" set 1 boot on
        progress_bar 2 4 "  Partitioning"

        if [[ "$DISK" == *"nvme"* ]] || [[ "$DISK" == *"mmcblk"* ]]; then
            ROOT_PART="${DISK}p1"
        else
            ROOT_PART="${DISK}1"
        fi

        sleep 1

        echo -e "\n  ${BLUE}●${NC} Formatting root partition (ext4)"
        mkfs.ext4 -F "$ROOT_PART" >/dev/null 2>&1
        progress_bar 3 4 "  Formatting "

        echo -e "\n\n  ${BLUE}●${NC} Mounting partition"
        mount "$ROOT_PART" /mnt
        progress_bar 4 4 "  Mounting   "
    fi

    echo -e "\n\n  ${GREEN}✓${NC} Disk prepared successfully"
}

# Install base system
install_base() {
    step_header "Installing Base System"

    # Check network connectivity
    echo -n "  Checking network... "
    if ! ping -c 1 -W 5 archlinux.org &>/dev/null; then
        echo -e "${YELLOW}not connected${NC}"
        echo "  Attempting to connect..."
        systemctl start NetworkManager 2>/dev/null || true
        sleep 3

        for iface in $(ip -o link show | awk -F': ' '{print $2}' | grep -v lo); do
            dhcpcd "$iface" 2>/dev/null &
        done
        sleep 5

        if ! ping -c 1 -W 5 archlinux.org &>/dev/null; then
            echo -e "  ${RED}✗${NC} No network connection"
            echo "    Use 'nmtui' or 'nmcli' to configure network"
            exit 1
        fi
    fi
    echo -e "${GREEN}connected${NC}"

    # Initialize pacman keyring
    echo -e "  ${BLUE}●${NC} Initializing package keyring..."
    pacman-key --init >/dev/null 2>&1
    pacman-key --populate archlinux >/dev/null 2>&1
    echo -e "  ${GREEN}✓${NC} Keyring initialized"

    # Base packages (always installed)
    local base_packages=(
        base base-devel
        linux linux-firmware linux-headers
        grub efibootmgr
        networkmanager
        zsh zsh-completions git
        pipewire pipewire-pulse wireplumber
        noto-fonts noto-fonts-emoji ttf-dejavu ttf-liberation ttf-fira-code
        sudo nano vim
        seatd
        ly
        gcc pkgconf
    )

    # Shell utilities
    local shell_utils=(
        gum zoxide fd ripgrep bat eza
    )

    # System utilities
    local system_utils=(
        htop btop tree fastfetch nnn
    )

    # Sandboxing (required for cvh-icons)
    local sandbox_packages=(
        bubblewrap libseccomp
    )

    # Compositor-specific packages
    local compositor_packages=()
    if [[ "$COMPOSITOR" == "niri" ]]; then
        compositor_packages=(
            niri
            xdg-desktop-portal-gnome
            xdg-desktop-portal-gtk
        )
    elif [[ "$COMPOSITOR" == "hyprland" ]]; then
        compositor_packages=(
            hyprland
            xdg-desktop-portal-wlr
            polkit-gnome
        )
    fi

    # Shared Wayland utilities
    local wayland_packages=(
        foot mako fuzzel
        grim slurp wl-clipboard
        wayland wayland-protocols xorg-xwayland
        brightnessctl
    )

    # Combine all packages
    local packages=(
        "${base_packages[@]}"
        "${shell_utils[@]}"
        "${system_utils[@]}"
        "${sandbox_packages[@]}"
        "${compositor_packages[@]}"
        "${wayland_packages[@]}"
    )

    echo
    echo -e "  ${BLUE}●${NC} Installing packages (this may take a while)..."
    echo -e "  ${DIM}────────────────────────────────────────────────────────${NC}"
    echo

    # Run pacstrap - show output directly
    if pacstrap -K /mnt "${packages[@]}"; then
        echo
        echo -e "  ${DIM}────────────────────────────────────────────────────────${NC}"
        echo -e "  ${GREEN}✓${NC} Base system installed"
    else
        echo
        echo -e "  ${RED}✗${NC} Package installation failed!"
        exit 1
    fi
}

# Generate fstab
generate_fstab() {
    step_header "Generating Filesystem Table"

    echo -n "  Creating /etc/fstab... "
    genfstab -U /mnt >> /mnt/etc/fstab
    echo -e "${GREEN}done${NC}"

    echo -e "\n  ${GREEN}✓${NC} Filesystem table generated"
}

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
    local current=0

    # Create a configuration script to run in chroot
    # Use /mnt/root/ which always exists after pacstrap
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
# CodeVerse Linux Ly Configuration

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
NAME="CodeVerse Linux"
PRETTY_NAME="CodeVerse Linux"
ID=cvh
ID_LIKE=arch
BUILD_ID=rolling
ANSI_COLOR="38;2;23;147;209"
HOME_URL="https://cvhlinux.org"
DOCUMENTATION_URL="https://wiki.cvhlinux.org"
LOGO=cvh-logo
EOF

# Set GRUB distributor name
sed -i 's/^GRUB_DISTRIBUTOR=.*/GRUB_DISTRIBUTOR="CodeVerse Linux"/' /etc/default/grub 2>/dev/null || \
    echo 'GRUB_DISTRIBUTOR="CodeVerse Linux"' >> /etc/default/grub

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

    Print { spawn "sh" "-c" "grim -g \\"\$(slurp)\\" ~/Pictures/screenshot-\$(date +%Y%m%d-%H%M%S).png"; }
}
NIRI_EOF
    chown -R $USERNAME:$USERNAME /home/$USERNAME/.config/niri

elif [[ "$COMPOSITOR" == "hyprland" ]]; then
    su - $USERNAME -c 'mkdir -p ~/.config/hypr'
    cat > /home/$USERNAME/.config/hypr/hyprland.conf << 'HYPR_EOF'
# CodeVerse Linux Hyprland Configuration

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
bind = , PRINT, exec, grim -g "\$(slurp)" ~/Pictures/Screenshots/screenshot-\$(date +%Y-%m-%d-%H-%M-%S).png
bind = \$mainMod, PRINT, exec, grim ~/Pictures/Screenshots/screenshot-\$(date +%Y-%m-%d-%H-%M-%S).png

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
    "$schema": "https://github.com/fastfetch-cli/fastfetch/raw/dev/doc/json_schema.json",
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

    # Copy custom CVH packages from ISO to installed system
    echo -e "  ${BLUE}●${NC} Copying CVH custom packages from ISO..."

    mkdir -p /mnt/var/cache/pacman/cvh-packages
    if [[ -d /opt/cvh-repo ]] && ls /opt/cvh-repo/*.pkg.tar.zst >/dev/null 2>&1; then
        cp /opt/cvh-repo/*.pkg.tar.zst /mnt/var/cache/pacman/cvh-packages/ 2>/dev/null || true
        echo -e "  ${GREEN}✓${NC} CVH packages copied ($(ls /opt/cvh-repo/*.pkg.tar.zst 2>/dev/null | wc -l) packages)"
    else
        echo -e "  ${YELLOW}⚠${NC}  CVH packages not found on ISO"
    fi

    # Create working mirrorlist for installed system
    echo -e "  ${BLUE}●${NC} Creating package mirrorlist..."
    mkdir -p /mnt/etc/pacman.d
    cat > /mnt/etc/pacman.d/mirrorlist << 'MIRRORLIST_EOF'
# Arch Linux mirrorlist - CodeVerse Linux
# Israeli mirrors
Server = https://mirror.isoc.org.il/pub/archlinux/$repo/os/$arch
Server = https://archlinux.mivzakim.net/$repo/os/$arch
# Global mirrors
Server = https://geo.mirror.pkgbuild.com/$repo/os/$arch
Server = https://mirrors.kernel.org/archlinux/$repo/os/$arch
Server = https://mirror.rackspace.com/archlinux/$repo/os/$arch
MIRRORLIST_EOF
    echo -e "  ${GREEN}✓${NC} Mirrorlist created"

    # Ensure pacman.conf has repository configuration
    echo -e "  ${BLUE}●${NC} Configuring package repositories..."
    if [[ -f /mnt/etc/pacman.conf ]]; then
        # Check if repos are already configured
        if ! grep -q "^\[core\]" /mnt/etc/pacman.conf; then
            cat >> /mnt/etc/pacman.conf << 'PACMAN_REPOS_EOF'

[core]
Include = /etc/pacman.d/mirrorlist

[extra]
Include = /etc/pacman.d/mirrorlist
PACMAN_REPOS_EOF
            echo -e "  ${GREEN}✓${NC} Repositories configured"
        else
            echo -e "  ${GREEN}✓${NC} Repositories already configured"
        fi
    fi

    chmod +x /mnt/root/configure.sh

    # Verify script was created
    if [[ ! -f /mnt/root/configure.sh ]]; then
        echo -e "\n  ${RED}✗${NC} Failed to create configuration script!"
        exit 1
    fi

    # Run configuration with progress
    for ((i=0; i<total; i++)); do
        progress_bar $((i+1)) $total "  Configuring"
        printf " ${DIM}%s${NC}" "${tasks[$i]}"
        sleep 0.5
    done

    echo -e "\n\n  ${BLUE}●${NC} Running system configuration..."
    arch-chroot /mnt /bin/bash /root/configure.sh
    rm -f /mnt/root/configure.sh

    # Install GRUB bootloader (run separately for better error handling)
    echo -e "\n\n  ${BLUE}●${NC} Installing GRUB bootloader..."
    if [[ "$BOOT_MODE" == "uefi" ]]; then
        arch-chroot /mnt grub-install --target=x86_64-efi --efi-directory=/boot/efi --bootloader-id=CVH
    else
        arch-chroot /mnt grub-install --target=i386-pc "$DISK"
    fi

    echo -e "  ${BLUE}●${NC} Generating GRUB config..."
    arch-chroot /mnt grub-mkconfig -o /boot/grub/grub.cfg

    echo -e "\n  ${GREEN}✓${NC} System configured"
}

# Set passwords
set_passwords() {
    step_header "Setting Passwords"

    echo -e "  ${BOLD}Set root password:${NC}"
    arch-chroot /mnt passwd root

    echo
    echo -e "  ${BOLD}Set password for $USERNAME:${NC}"
    arch-chroot /mnt passwd "$USERNAME"

    echo -e "\n  ${GREEN}✓${NC} Passwords set"
}

# Finish installation
finish_installation() {
    step_header "Finishing Installation"

    echo -n "  Syncing filesystems... "
    sync
    echo -e "${GREEN}done${NC}"

    echo -n "  Unmounting partitions... "
    umount -R /mnt
    echo -e "${GREEN}done${NC}"

    show_overall_progress

    echo
    echo -e "${BOLD}${GREEN}"
    cat << 'EOF'
  ╔════════════════════════════════════════════════════════════════╗
  ║                                                                ║
  ║              Installation Complete!                            ║
  ║                                                                ║
  ╚════════════════════════════════════════════════════════════════╝
EOF
    echo -e "${NC}"

    echo -e "  ${BOLD}System Details:${NC}"
    echo -e "    Username:  ${CYAN}$USERNAME${NC}"
    echo -e "    Hostname:  ${CYAN}$HOSTNAME${NC}"
    echo -e "    Timezone:  ${CYAN}$TIMEZONE${NC}"
    echo -e "    Boot Mode: ${CYAN}$BOOT_MODE${NC}"
    echo

    echo -e "  ${BOLD}After Reboot:${NC}"
    echo -e "    1. ${GREEN}Ly display manager${NC} will appear on boot"
    echo -e "    2. Select ${CYAN}$COMPOSITOR${NC} session"
    echo -e "    3. Enter your username and password"
    echo -e "    4. Press ${CYAN}Mod+Return${NC} to open terminal"
    echo -e "    5. Press ${CYAN}Mod+D${NC} to open app launcher (cvh-fuzzy)"
    echo
    echo -e "  ${DIM}CVH Tools: cvh-fuzzy (launcher), cvh-icons (desktop icons)${NC}"
    echo

    read -r -p "  Press Enter to reboot..." _ || true
    reboot
}

# Main installation flow
main() {
    check_root
    show_welcome
    detect_boot_mode
    select_keyboard
    select_timezone
    select_compositor
    select_disk
    set_hostname
    create_user_config
    partition_disk
    install_base
    generate_fstab
    configure_system
    set_passwords
    finish_installation
}

# Run main
main "$@"
