# Contributing

Thanks for contributing to CodeVerse Linux (CodeVerse Linux).

## Setup (build environment)

This project builds an ArchISO and is easiest to work on from an **Arch-based host**.

Required tools for ISO builds:

- `archiso` (provides `mkarchiso`)
- `squashfs-tools` (provides `mksquashfs`)
- `libisoburn` (provides `xorriso`)
- `grub` (provides `grub-mkrescue`)

Install (Arch):

- `sudo pacman -S archiso squashfs-tools libisoburn grub`

If you touch `src/` (Rust tools), install Rust via rustup.

## Common tasks

- Build packages: `./scripts/build-packages.sh`
- Build ISO: `./scripts/build-iso.sh`

More detail: [docs/BUILD.md](docs/BUILD.md).

## Testing changes

Recommended: test in a VM first.

- Boot the ISO in QEMU/virt-manager
- Smoke test:
  - Boot to live environment
  - Run `cvh-install` and complete an install on a disposable virtual disk

VM walkthrough: [docs/TESTING.md](docs/TESTING.md).

## Submitting a PR

1. Fork the repo and create a feature branch.
2. Keep PRs focused (one logical change).
3. Describe what changed and how it was tested.
4. Submit a pull request using the PR template.
