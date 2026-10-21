# CMF Buds Pro 2 — Smart Dial Control System

Turn your **CMF Buds Pro 2** smart dial case into a context-aware, beautifully animated radial controller for **Linux, macOS, and Windows** — inspired by **[KnifeLemon/Filee](https://github.com/KnifeLemon/Filee)** and the Nothing OS aesthetic.

- **100% Dual-Pairing Preserved**: Keeps your phone connection and the Nothing X app active simultaneously.
- **Click the Dial**: An animated, glowing Donut Radial HUD pops up on screen with options tailored to your currently focused application.
- **Rotate the Dial**: Smoothly cycles options with audio beeps in your earbuds while your media volume remains locked.
- **Click Again**: Executes the action (e.g. `Copy`/`Paste` in Files, `Seek`/`Fullscreen` in YouTube, `Format` in VS Code, or opens the Volume/Brightness gauge).
- **System Tray Daemon**: Runs quietly in your system tray / top panel with quick controls, radial toggles, and settings customizer.
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
│   │   ├── hud/            # DonutRadialHUD (Framer Motion circular wheel)
│   │   └── settings/       # SettingsWindow (Slice editor & profile manager)
│   ├── data/               # Default wheel slices & actions
│   └── types/              # TypeScript profile schemas
├── src-tauri/              # Backend Daemon (Rust + Tauri v2)
│   ├── Cargo.toml          # Native dependencies & conditional targets
│   └── src/
│       ├── lib.rs          # System tray, IPC commands & app coordinator
│       ├── dial_backend.rs # Hardware dial listener & rotary tracker
│       ├── virtual_input.rs# Cross-platform hotkey & media key dispatcher
│       └── context_detector.rs # Active foreground window tracking
└── temp/                   # [Gitignored] Reference prototypes & scripts
```

---

## Cross-Platform Roadmap & Status

| Platform | Dial / AVRCP Interception | Volume Control | Brightness Control | Context Detection | Status |
|---|---|---|---|---|---|
| **Linux (X11 / Wayland)** | BlueZ D-Bus + `evdev` grabber | WirePlumber (`wpctl`) / PulseAudio | `/sys/class/backlight` + GNOME Mutter D-Bus | AT-SPI2 / X11 EWMH | **Fully Functional** |
| **Windows** | Win32 RawInput / AVRCP | WASAPI (`IAudioEndpointVolume`) | WMI / Win32 Monitor API | Win32 `GetForegroundWindow` | **In Development** |
| **macOS** | CoreBluetooth / IOHIDManager | CoreAudio HAL | DisplayServices / IOKit | Cocoa `NSWorkspace` | **In Development** |

---

## Context-Aware Profiles

The dial automatically detects your active foreground application and adapts its options in real-time:

| Foreground App | Profile | Slices on the Wheel |
|---|---|---|
| **Files / Nautilus / Dolphin** | **📁 Files & Folders** | 📋 Copy (`Ctrl+C`), 📥 Paste (`Ctrl+V`), 🗑️ Trash (`Delete`), 📁 New Folder (`Ctrl+Shift+N`), 🏷️ Rename (`F2`), 💻 Terminal |
| **YouTube / Firefox / Chrome / Zen** | **🎬 Video & Media** | ⏯️ Play/Pause, ⏩ Seek +10s (`Right`), ⏪ Seek -10s (`Left`), 🖥️ Fullscreen (`F`), 🔇 Mute (`M`), 🔊 Volume Gauge |
| **VS Code / Cursor / Zed** | **💻 Code & Editor** | 💾 Save All (`Ctrl+S`), ✨ Format (`Shift+Alt+F`), 🔍 Find in Files (`Ctrl+Shift+F`), 💬 Comment (`Ctrl+/`), 💻 Terminal (`Ctrl+\``), 🔄 Git |
| **Desktop / General Apps** | **⚙️ System Controls** | 🔊 Volume Adjust, ☀️ Brightness Adjust, ⏯️ Play/Pause, ⏭️ Next Track, ⏮️ Prev Track, 🔇 Mute Toggle |

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

# 3. Build standalone production binary / installer
pnpm tauri build
```

---

## Customizing Profiles

Profiles and wheel slices can be customized via the **Settings Window** (accessible from the system tray menu) or by editing `config/profiles.json`:

- Reorder, add, or delete wheel slices.
- Assign custom hotkeys (`Ctrl+C`, `Shift+Alt+F`), media keys, or terminal commands.
- Modify inner and outer radius, slice colors, and animation speed.
- Changes saved to `config/profiles.json` take effect immediately on the live radial dial without restarting.
