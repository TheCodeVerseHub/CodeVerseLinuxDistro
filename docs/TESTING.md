# Testing

The safest workflow is to test in a VM before trying real hardware.

## VM smoke test (recommended)

1. Build an ISO (see [docs/BUILD.md](BUILD.md)).
2. Create a VM (virt-manager/QEMU) with:
   - UEFI firmware if possible
   - A **disposable** virtual disk (the installer wipes it)
3. Boot the ISO.
4. In the live environment, run:
   - `sudo cvh-install`
5. Complete an install and reboot into the installed system.

## What to verify

- ISO boots (UEFI and/or BIOS)
- Installer completes without errors
- Installed system boots
- Wayland session starts (niri or Hyprland)
- Network works (basic connectivity)

If a test fails, include logs in your issue:

- `journalctl -b`
- Any installer output shown on screen