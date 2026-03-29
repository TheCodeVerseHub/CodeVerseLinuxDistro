# Architecture

CVH Linux is assembled as an ArchISO profile with a small set of custom packages and synced user/system configuration.

## Key directories

- `iso/`: the ArchISO profile (packages list, `airootfs/`, boot configs)
- `scripts/`: build entrypoints (`build-iso.sh`, `build-packages.sh`)
- `configs/`: GRUB theme and user configs that are copied into the ISO skeleton
- `src/`: Rust sources for custom utilities
- `pkgbuild/`: PKGBUILDs used to package custom utilities
- `repo/`: local pacman repository artifacts (built packages)

## Build flow (high level)

1. `scripts/build-packages.sh` builds custom packages (Rust binaries) and produces pacman packages.
2. `scripts/build-iso.sh` prepares a working copy of `iso/` and syncs:
   - GRUB theme from `configs/grub/themes/`
   - user setup configs from `configs/setup-configs/` into `iso/airootfs/etc/skel`
   - built packages into `iso/airootfs/opt/cvh-repo` (for offline use)
3. `mkarchiso` builds the final ISO from the prepared profile.

## Installer

The live ISO provides `cvh-install`, which is a modular installer sourcing shell modules under `/usr/lib/cvh-install` in the live environment.