# CodeVerse Linux (CodeVerse Linux)

CodeVerse Linux (CodeVerse Linux) is a **community-driven, Arch-based Linux distribution** with a **Wayland-first** focus. The repository contains an **ArchISO profile** (`iso/`), build tooling (`scripts/`), curated configs (`configs/`), and a small set of custom packages (`src/`, `pkgbuild/`, `repo/`).

## Target

- **Architecture:** x86_64
- **Recommended for:** VM testing while the project is evolving
- **Also intended for:** real hardware (use caution; the installer wipes the selected disk)

## Getting Started (Beginner-Friendly)
 
This section walks you through everything from scratch. No prior Arch Linux or ISO experience needed.
 
### What is an ISO?
 
An ISO file is a complete, bootable disk image — think of it as a virtual CD/USB that contains a live operating system. To use CVH Linux, you first need to *build* this ISO file from the source code in this repo, then either:
- Flash it to a USB drive and boot your computer from it, **or**
- Attach it to a Virtual Machine (VM) like VirtualBox or QEMU

### Step 1: Set Up Your Build Machine
 
> **What this means:** You need an existing Arch-based Linux system (like Arch Linux, EndeavourOS, or Manjaro) to build the ISO. Building the ISO is currently supported only on Arch-based systems.
 
If you don’t have an Arch-based system, you can use a virtual machine (VM) with Arch Linux for building.
 
Make sure the following tools are installed on your build machine:
 
 ```bash
sudo pacman -S archiso git
```

### Step 2: Clone This Repository
 
 ```bash
git clone https://github.com/TheCodeVerseHub/CodeVerseLinuxDistro.git
cd CodeVerseLinuxDistro
```

### Step 3: Build the ISO
 
Run the build script:
 ```bash
./scripts/build-iso.sh
 ```

This will take several minutes — it downloads packages and assembles the full system image. When it finishes, you'll find the `.iso` file inside the `out/` folder.
 
> For more detailed build options and troubleshooting, see [docs/BUILD.md](docs/BUILD.md).

### Step 4: Boot the ISO
 This will start a live system that runs directly from the ISO without installing anything.
You have two options depending on whether you're using a real machine or a VM.
 
#### Option A: Real Hardware (USB Drive)
 
>  **Warning:** The installer will wipe the disk you select. Back up any important data first.

Flash the ISO to a USB drive. Replace `/dev/sdX` with your actual USB device (use `lsblk` to find it):
 
 ```bash
sudo dd if=out/codeverse-linux-*.iso of=/dev/sdX bs=4M status=progress oflag=sync
```
 
Then plug the USB into your target machine and boot from it (usually by pressing F2, F12, or Del during startup to open the boot menu).
 
#### Option B: Virtual Machine — VirtualBox
 
1. Open VirtualBox and click **New**
2. Set type to **Linux**, version to **Arch Linux (64-bit)**
3. Allocate at least **2 GB RAM** and **20 GB disk space**
4. Under **Settings → Storage**, attach the `.iso` file as an optical drive
5. Start the VM — it will boot directly into the CVH Linux live environment
 
#### Option C: Virtual Machine — QEMU
 
```bash
qemu-system-x86_64 \
  -m 2G \
  -cdrom out/codeverse-linux-*.iso \
  -boot d \
  -enable-kvm
```
 
> Remove `-enable-kvm` if your system doesn't support KVM (you'll see an error if so).
 
 ### Step 5: Run the Installer
 
Once booted into the live environment, open a terminal and run:
 
 ```bash
sudo cvh-install
 ```

The installer is interactive — it will ask you which disk to install to and walk you through the setup. Follow the on-screen prompts.
 
>**Reminder:** The installer will partition and format the disk you choose. Make sure you select the correct one.
 

## Screenshots

Add screenshots to this section as the UI stabilizes.

- (placeholder) Desktop
- (placeholder) Installer

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for environment setup, testing, and PR guidelines.

## Repo Layout (High Level)
 
| Path | Description |
|---|---|
| `iso/` | ArchISO profile (packages list, `airootfs/`, bootloader config) |
| `scripts/` | Build ISO, build packages, legacy/standalone installer script |
| `configs/` | GRUB theme + user setup configs (niri, waybar, rofi, etc.) |
| `src/` + `pkgbuild/` + `repo/` | Custom packages and local pacman repo artifacts |

## License

GPL-3.0-only (see [LICENSE](LICENSE)).
