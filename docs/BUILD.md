# Building the ISO

This repo builds a bootable live ISO using ArchISO (`mkarchiso`) and the profile in `iso/`.

## Prerequisites (Arch-based host)

- `archiso` (mkarchiso)
- `squashfs-tools` (mksquashfs)
- `libisoburn` (xorriso)
- `grub` (grub-mkrescue)
- `git`

Install:

- `sudo pacman -S archiso squashfs-tools libisoburn grub git`

## Build

From the repo root:

1. (Optional) build custom packages:
   - `./scripts/build-packages.sh`
2. Build the ISO:
   - `./scripts/build-iso.sh`

Artifacts:

- ISO output directory: `out/`
- Build workspace (defaults to): `.build/`

Notes:

- `scripts/build-iso.sh` uses `sudo mkarchiso ...` and will prompt for your password.
- The build script syncs setup configs from `configs/setup-configs/` into the ISO skeleton.