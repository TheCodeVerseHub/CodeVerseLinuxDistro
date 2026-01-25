#!/bin/bash
# CVH Linux Installer
# Run from live environment to install CVH Linux to disk

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
    echo -e "    ${GREEN}●${NC} Niri Wayland compositor"
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

    # Package list
    local packages=(
        base base-devel
        linux linux-firmware linux-headers
        grub efibootmgr
        networkmanager
        zsh zsh-completions git
        niri foot mako fuzzel
        pipewire pipewire-pulse wireplumber
        grim slurp wl-clipboard
        noto-fonts ttf-dejavu
        sudo nano vim
        seatd
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
    cat > /mnt/tmp/configure.sh << CONFIGURE_SCRIPT
#!/bin/bash
set -e

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

# Enable services
systemctl enable NetworkManager >/dev/null 2>&1
systemctl enable systemd-timesyncd >/dev/null 2>&1
systemctl enable seatd >/dev/null 2>&1

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

# Ensure zsh is in /etc/shells
grep -q "/usr/bin/zsh" /etc/shells || echo "/usr/bin/zsh" >> /etc/shells
grep -q "/bin/zsh" /etc/shells || echo "/bin/zsh" >> /etc/shells

# Create user (add to seat group for seatd)
useradd -m -G wheel,audio,video,input,seat -s /usr/bin/zsh $USERNAME

# Ensure shell is set (in case useradd didn't work)
chsh -s /usr/bin/zsh $USERNAME

# Configure sudo
echo "%wheel ALL=(ALL:ALL) ALL" > /etc/sudoers.d/wheel
chmod 440 /etc/sudoers.d/wheel

# Set up Oh My Zsh
su - $USERNAME -c 'git clone --depth=1 https://github.com/ohmyzsh/ohmyzsh.git ~/.oh-my-zsh 2>/dev/null' || true
su - $USERNAME -c 'cp ~/.oh-my-zsh/templates/zshrc.zsh-template ~/.zshrc 2>/dev/null' || true

# Create Niri config
su - $USERNAME -c 'mkdir -p ~/.config/niri'
cat > /home/$USERNAME/.config/niri/config.kdl << 'NIRI_EOF'
input {
    keyboard {
        xkb { layout "us" }
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
}
NIRI_EOF
chown -R $USERNAME:$USERNAME /home/$USERNAME/.config

# Create .zshrc
cat > /home/$USERNAME/.zshrc << 'ZSHRC_EOF'
export ZSH="\$HOME/.oh-my-zsh"
ZSH_THEME="robbyrussell"
plugins=(git sudo history)
[[ -f \$ZSH/oh-my-zsh.sh ]] && source \$ZSH/oh-my-zsh.sh

export EDITOR="nano"
export QT_QPA_PLATFORM="wayland"
export MOZ_ENABLE_WAYLAND="1"

alias ls='ls --color=auto'
alias ll='ls -la'

# Alias to start Niri
alias start-niri='niri-session'

# Uncomment below to auto-start Niri on tty1
# if [[ -z "\$WAYLAND_DISPLAY" ]] && [[ "\$XDG_VTNR" -eq 1 ]]; then
#     exec niri-session
# fi
ZSHRC_EOF
chown $USERNAME:$USERNAME /home/$USERNAME/.zshrc
CONFIGURE_SCRIPT

    chmod +x /mnt/tmp/configure.sh

    # Run configuration with progress
    for ((i=0; i<total; i++)); do
        progress_bar $((i+1)) $total "  Configuring"
        printf " ${DIM}%s${NC}" "${tasks[$i]}"
        sleep 0.5
    done

    arch-chroot /mnt /tmp/configure.sh
    rm /mnt/tmp/configure.sh

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
    echo -e "    1. Log in with your username and password"
    echo -e "    2. Niri (Wayland) starts automatically on tty1"
    echo -e "    3. Press ${CYAN}Mod+Return${NC} to open terminal"
    echo -e "    4. Press ${CYAN}Mod+D${NC} to open app launcher"
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
