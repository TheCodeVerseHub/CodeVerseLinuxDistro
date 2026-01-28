#!/bin/bash
# CVH Linux Installer - Finalize Module
# Password setting, cleanup, and reboot

# Set passwords
set_passwords() {
    step_header "Setting Passwords"

    gum style --bold "  Set root password:"
    arch-chroot /mnt passwd root

    echo
    gum style --bold "  Set password for $USERNAME:"
    arch-chroot /mnt passwd "$USERNAME"

    gum style --foreground 82 "  ✓ Passwords set"
}

# Finish installation
finish_installation() {
    step_header "Finishing Installation"

    gum spin --spinner dot --title "Syncing filesystems..." -- sync

    unmount_all

    show_overall_progress

    show_completion

    # System details box
    gum style --border rounded --padding "1 2" --margin "1 0" \
        "$(gum style --bold 'System Details')" \
        "Username:  $USERNAME" \
        "Hostname:  $HOSTNAME" \
        "Timezone:  $TIMEZONE" \
        "Boot Mode: $BOOT_MODE"

    # After reboot instructions
    gum style --border rounded --padding "1 2" --margin "1 0" \
        "$(gum style --bold 'After Reboot')" \
        "1. Ly display manager will appear on boot" \
        "2. Select $COMPOSITOR session" \
        "3. Enter your username and password" \
        "4. Press Mod+Return to open terminal" \
        "5. Press Mod+D to open app launcher (cvh-fuzzy)"

    gum style --faint "CVH Tools: cvh-fuzzy (launcher), cvh-icons (desktop icons)"
    echo

    gum confirm "Ready to reboot?" --affirmative "Reboot Now" --negative "Cancel" && reboot
}
