# CMF Buds Pro 2 — Smart Dial Control System

Turn your **CMF Buds Pro 2** smart dial case into a context-aware, beautifully animated radial controller for **Linux, macOS, and Windows** — inspired by **[KnifeLemon/Filee](https://github.com/KnifeLemon/Filee)** and the Nothing OS aesthetic.

- **100% Dual-Pairing Preserved**: Keeps your phone connection and the Nothing X app active simultaneously.
- **Direct Launch & System Tray Docking**: Launches directly into the Settings & Customizer window on startup. Closing the window minimizes seamlessly to the system tray background daemon.
- **Nothing OS Typography**: Fully styled with authentic Nothing OS dot-matrix typography (`Ndot57`) across all HUD dials, settings panels, slice inspectors, and configuration cards.
- **Custom Dials Creator**: Create custom dial profiles from scratch with custom shortcut triggers, import actions across any dial, and configure dynamic window title/class pattern matching.
- **Global Activation Shortcut**: Full keyboard-only radial control for users without CMF Buds hardware. Equipped with a conflict-free default (`Super+Shift+D`) and an in-app interactive shortcut rebinder.
- **Custom Nothing App Icon**: Distinctive dot-matrix circular emblem integrated into taskbars, window decorations, system notifications, and system tray.
- **Click the Dial / Press Shortcut**: An animated, glowing Donut Radial HUD pops up on screen with options tailored to your currently focused application.
- **Home Dial Option**: Every dial menu features a `HOME` option that opens the interactive Home Dial switcher where you can manually choose between any dial menu (System, Media, Code, Files, Browser, Design, or custom profiles).
- **Rotate the Dial / Arrow Keys**: Smoothly cycles options with audio feedback while your media volume remains locked.
- **Click Again / Enter**: Executes the action (e.g. `Copy`/`Paste` in Files, `Seek`/`Fullscreen` in YouTube, `Format` in VS Code, or opens the Volume/Brightness gauge).
- **System Tray Daemon**: Runs quietly in your system tray / top panel with clean, emoji-free controls, radial toggles, and settings customizer.
- **Built with Tauri v2 + Rust + React**: Native performance, minimal memory footprint, and cross-platform architecture.

---

## Architecture Overview

```
dial-control/
├── assets/                 # Desktop & system tray icons
├── config/                 # Declarative profiles.json configuration
├── docs/                   # Protocol & architectural specifications
├── public/                 # Typography (NType82, Ndot57) & assets
├── scripts/                # Platform helper scripts (e.g. udev rules)
├── src/                    # Frontend UI (React 18, TypeScript, Tailwind CSS)
│   ├── components/
│   │   ├── hud/            # DonutRadialHUD (Framer Motion circular wheel & Home Dial)
│   │   └── settings/       # SettingsWindow (Slice editor & profile manager)
│   ├── data/               # Default wheel slices & actions
│   └── types/              # TypeScript profile schemas
├── src-tauri/              # Backend Daemon (Rust + Tauri v2)
│   ├── Cargo.toml          # Native dependencies & conditional targets
│   └── src/
│       ├── lib.rs          # System tray, IPC commands & app coordinator
│       └── platform/       # Modular OS Abstractions (Linux, Windows, macOS)
│           ├── mod.rs      # Unified provider switcher
│           ├── traits.rs   # Core platform traits
│           ├── linux/      # WirePlumber, Mutter, uinput, BlueZ, evdev & D-Bus Context
│           ├── windows/    # WASAPI, Win32 & RawInput stubs
│           └── macos/      # CoreAudio, IOKit & Cocoa stubs
└── temp/                   # [Gitignored] Reference prototypes & scripts
```

---

## Cross-Platform Roadmap & Status

| Platform | Dial / AVRCP Interception | Volume Control | Brightness Control | Context Detection | Status |
|---|---|---|---|---|---|
| **Linux (X11 / Wayland)** | BlueZ D-Bus + `evdev` grabber | WirePlumber (`wpctl`) / PulseAudio | `/sys/class/backlight` + GNOME Mutter D-Bus | GNOME FocusedWindow D-Bus + Sway/Hyprland/KDE/X11 | **Fully Functional** |
| **Windows** | Win32 RawInput / AVRCP | WASAPI (`IAudioEndpointVolume`) | WMI / Win32 Monitor API | Win32 `GetForegroundWindow` | **In Development** |
| **macOS** | CoreBluetooth / IOHIDManager | CoreAudio HAL | DisplayServices / IOKit | Cocoa `NSWorkspace` | **In Development** |

---

## Context-Aware Profiles & Home Dial

The dial automatically detects your active foreground application and adapts its options in real-time. In addition, you can rotate to **HOME** in any dial menu to manually open the **Home Dial** launcher and choose between different profiles:

| Foreground App | Profile | Slices on the Wheel |
|---|---|---|
| **Files / Nautilus / Dolphin** | **📁 Files & Folders** | 📋 Copy (`Ctrl+C`), 📥 Paste (`Ctrl+V`), 🗑️ Trash (`Delete`), 📁 New Folder (`Ctrl+Shift+N`), 🏷️ Rename (`F2`), 💻 Terminal, 🏠 Home Dial |
| **YouTube / Firefox / Chrome / Zen** | **🎬 Video & Media** | ⏯️ Play/Pause, ⏩ Seek +10s (`Right`), ⏪ Seek -10s (`Left`), 🖥️ Fullscreen (`F`), 🔇 Mute (`M`), 🔊 Volume Gauge, 🏠 Home Dial |
| **VS Code / Cursor / Zed** | **💻 Code & Editor** | 💾 Save All (`Ctrl+S`), ✨ Format (`Shift+Alt+F`), 🔍 Find in Files (`Ctrl+Shift+F`), 💬 Comment (`Ctrl+/`), 💻 Terminal (`Ctrl+\``), 🔄 Git, 🏠 Home Dial |
| **Desktop / General Apps** | **⚙️ System Controls** | 🔊 Volume Adjust, ☀️ Brightness Adjust, ⏯️ Play/Pause, ⏭️ Next Track, ⏮️ Prev Track, 🔇 Mute Toggle, 🏠 Home Dial |
| **Any Screen (via Home)** | **🏠 Home Dial Switcher** | ⚙️ System, 🎬 Media, 💻 Code, 📁 Files, 🌐 Browser, 🎨 Design (plus any custom user profiles) |

---

## Pre-Built Distribution Packages

Dial Control provides native, pre-assembled packages for all major Linux distributions in [`dist-packages/`](file:///home/yuvraj/Documents/Code_Projects/dial-control/dist-packages/):

| Distribution | Package Format | Installation Command |
|---|---|---|
| **Debian / Ubuntu 24.04+ / Pop!_OS / Mint** | `.deb` | `sudo dpkg -i dist-packages/dial-control_1.0.0_amd64.deb` |
| **Fedora 40/41 / Nobara / RHEL / openSUSE** | `.rpm` | `sudo dnf install ./dist-packages/dial-control-1.0.0-1.x86_64.rpm` |
| **Arch Linux / Manjaro / EndeavourOS** | `.pkg.tar.zst` | `sudo pacman -U ./dist-packages/dial-control-1.0.0-1-x86_64.pkg.tar.zst` |
| **Any Linux (Portable)** | `.AppImage` | `chmod +x dist-packages/Dial-Control-1.0.0-amd64.AppImage && ./dist-packages/Dial-Control-1.0.0-amd64.AppImage` |

### What is Bundled Inside the Native Packages?

Each native package (`.deb`, `.rpm`, and Arch package) installs a complete system integration with zero manual browsing or repository hunting required:
- **Application Binary**: Installed to `/usr/bin/dial-control`.
- **Desktop Entry & Icon**: Registered under `/usr/share/applications/` and `/usr/share/icons/hicolor/512x512/apps/`.
- **Hardware Access Rules (`udev`)**: Installs `/usr/lib/udev/rules.d/99-dial-control.rules` to give the logged-in desktop user non-root access to `/dev/uinput` (virtual keystroke emulation) and the CMF Buds Pro 2 rotary input events.
- **GNOME Shell Focused Window Extension**: Installs the `focused-window-dbus@flexagoon.com` extension system-wide to `/usr/share/gnome-shell/extensions/` to enable Wayland context detection.
- **Post-Install Script**: Automatically reloads udev rules (`udevadm control --reload-rules && udevadm trigger`), refreshes desktop and icon databases, and enables the GNOME extension.

---

## Distribution Compatibility & System Requirements

### Compatibility Matrix

| Distribution / Environment | Status | Details & Requirements |
|---|---|---|
| **Ubuntu 24.04 LTS (Noble) & 24.10+** | **Supported** | Native `.deb` works out of the box. GNOME Shell 46+. |
| **Fedora 40, 41 & Nobara** | **Supported** | Native `.rpm` works out of the box. GNOME Shell 46/47+. |
| **Arch Linux / Manjaro / EndeavourOS** | **Supported** | Native `.pkg.tar.zst` works out of the box. |
| **openSUSE Tumbleweed** | **Supported** | Native `.rpm` works out of the box. |
| **KDE Plasma (Any Modern Distro)** | **Supported** | Natively queries active window via `kdotool`. Zero extensions needed. |
| **Hyprland / Sway (Wayland)** | **Supported** | Natively queries active window via `hyprctl` / `swaymsg`. Zero extensions needed. |
| **X11 / XWayland** | **Supported** | Natively queries active window via `xdotool`. |
| **Ubuntu 22.04 LTS / Debian 12** | **Incompatible** | Requires `GLIBC >= 2.39`. See note below for older distros. |

### Technical Details & First-Time Setup Notes

1. **GLIBC Requirement (`GLIBC_2.39`)**:
   - The release binaries are compiled on Fedora/Nobara with `GLIBC_2.39`. Modern Linux releases (Ubuntu 24.04+, Fedora 40+, Arch) satisfy this requirement.
   - On older distributions (Ubuntu 22.04 LTS with GLIBC 2.35 or Debian 12 with GLIBC 2.36), the binary will not launch. To target older distributions, build the release binary inside an `ubuntu:22.04` container.
2. **GNOME Wayland First-Time Session Restart**:
   - GNOME Shell on Wayland cannot reload extensions in a live session (`Alt+F2 -> r` is disabled on Wayland).
   - After installing Dial Control on a **GNOME Wayland** system for the first time, **log out and log back in once** (or reboot) so GNOME Shell loads the newly placed system extension.
   - *Graceful fallback*: If the extension is not yet loaded, Dial Control will not crash; it will operate normally using the default dial profile.
3. **Bluetooth Connection Order**:
   - If your CMF Buds Pro 2 were already connected via Bluetooth *before* running the package installation, disconnect and reconnect the earbuds once over Bluetooth so `systemd-logind` attaches the new `uaccess` permissions to the input node.
4. **AppImage Considerations**:
   - The standalone `.AppImage` runs without root privileges and therefore cannot automatically place udev rules into `/usr/lib/udev/rules.d/` or system extensions into `/usr/share/gnome-shell/extensions/`.
   - When using the AppImage, either run `bash scripts/setup_udev.sh` once or add your user to the `input` group (`sudo usermod -aG input $USER`).

---

## Getting Started

### Prerequisites

- **Node.js** (v18+) and **pnpm** (or npm/yarn)
- **Rust** (stable toolchain)
- **Linux Packages** (if running on Linux):
  ```bash
  # Debian/Ubuntu
  sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev
  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl libappindicator-gtk3 librsvg
  # Fedora
  sudo dnf install webkit2gtk4.1-devel openssl-devel libappindicator-gtk3-devel librsvg2-devel
  ```

### Permissions Setup (Linux Non-Root Execution)

To allow Dial Control to intercept dial clicks and emulate keys without requiring `sudo`:

```bash
bash scripts/setup_udev.sh
```

This installs `/etc/udev/rules.d/99-cmf-dial.rules` and grants your user account `uaccess` to the CMF Buds AVRCP input events.

---

## Development & Building

```bash
# 1. Install dependencies
pnpm install

# 2. Run in development mode (hot reloading UI + Rust backend)
pnpm tauri dev

# 3. Build standalone production binary (embeds frontend into binary)
pnpm tauri build --no-bundle
# (or: npx @tauri-apps/cli build --no-bundle)

# 4. Copy to launcher bin/ (optional for desktop integration)
cp src-tauri/target/release/dial-control bin/dial-control

# 5. Build all multi-distribution packages (.deb, .rpm, AppImage, Arch Linux)
bash scripts/build-packages.sh
```

---

## Customizing Profiles & Global Shortcuts

Profiles, wheel slices, and activation triggers can be customized via the **Settings Window** (which opens on launch or via the system tray menu) or by editing `config/profiles.json`:

- **Create Custom Dials**: Add dedicated radial profiles tailored to any application or workflow with custom icons and title/WM_CLASS window matching rules.
- **Import Slices**: Pick and reuse actions from any existing dial (e.g. VS Code shortcuts or media controls) directly into your custom dial with a single click.
- **Reorder & Edit Slices**: Adjust wheel slice order, customize labels, reassign key combinations (`Ctrl+C`, `Shift+Alt+F`), media keys, or arbitrary shell commands.
- **Interactive Shortcut Recorder & Wayland IPC**: Record any global activation shortcut directly from your keyboard without manual typing. On Linux Wayland, automatically binds with GNOME compositor keybindings and triggers the radial HUD in < 2ms via a high-speed local IPC socket.
- **Radial Geometry & Aesthetics**: Modify inner/outer radius, donut thickness, accent glow colors, and rotation animations.
- **Instant Hot-Reload**: Changes saved in Settings take effect immediately on the live radial dial without restarting the daemon.
