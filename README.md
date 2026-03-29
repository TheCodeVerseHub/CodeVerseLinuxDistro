# CodeVerse Linux (CVH Linux)

CodeVerse Linux (CVH Linux) is a **community-driven, Arch-based Linux distribution** with a **Wayland-first** focus. The repository contains an **ArchISO profile** (`iso/`), build tooling (`scripts/`), curated configs (`configs/`), and a small set of custom packages (`src/`, `pkgbuild/`, `repo/`).

## Target

- **Architecture:** x86_64
- **Recommended for:** VM testing while the project is evolving
- **Also intended for:** real hardware (use caution; the installer wipes the selected disk)

## Install (from a built ISO)

1. Build the ISO (see [docs/BUILD.md](docs/BUILD.md)).
2. Write the ISO to USB (or attach it to your VM) and boot it.
3. In the live environment, run the installer:
   - `sudo cvh-install`

The installer is interactive and will partition/format the chosen disk.

## Build ISO (quick)

On an Arch-based build host:

- `./scripts/build-iso.sh`

Outputs land in `out/` by default. Full details: [docs/BUILD.md](docs/BUILD.md).

## Screenshots

Add screenshots to this section as the UI stabilizes.

- (placeholder) Desktop
- (placeholder) Installer

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, testing, and PR guidelines.

## Repo layout (high level)

- `iso/`: ArchISO profile (packages list, `airootfs/`, bootloader config)
- `scripts/`: build ISO, build packages, legacy/standalone installer script
- `configs/`: GRUB theme + user setup configs (niri, waybar, rofi, etc.)
- `src/` + `pkgbuild/` + `repo/`: custom packages and local pacman repo artifacts

## License

GPL-3.0-only (see [LICENSE](LICENSE)).
