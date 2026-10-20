# CMF Buds Pro 2 — Smart Dial Control System

Turn your **CMF Buds Pro 2** smart dial case into a context-aware, beautifully animated radial controller for Linux, Windows, and macOS — inspired by **[KnifeLemon/Filee](https://github.com/KnifeLemon/Filee)**.

- **100% Dual-Pairing Preserved**: Keeps your phone connection and the Nothing X app active simultaneously.
- **Click the Dial**: An animated, glowing Donut Radial HUD pops up on screen with options tailored to your currently focused application.
- **Rotate the Dial**: Smoothly cycles options with audio beeps in your earbuds while your media volume remains locked.
- **Click Again**: Executes the action (e.g. `Copy`/`Paste` in Files, `Seek`/`Fullscreen` in YouTube, `Format` in VS Code, or opens the Volume/Brightness gauge).
- **System Tray Daemon**: Runs quietly in your top panel with quick controls, profile switching, and settings launcher.
- **Applications Menu Integration**: Launch directly from GNOME "Show Applications" just like any native desktop app.

---

## Architecture Overview

```
dial-control/
├── assets/
│   └── dial-control.png        # Application desktop & tray icon
├── bin/
│   └── dial-control            # Main application launcher
├── config/
│   └── profiles.json           # Declarative JSON schema for wheel slices & app profiles
├── docs/
│   ├── CROSS_PLATFORM_SPEC.md  # Multi-OS architecture specification (Rust + Tauri v2)
│   ├── DEVELOPMENT_GUIDE.md    # Developer guide
│   └── PROTOCOL_SPEC.md        # Reverse-engineered Bluetooth wire protocol
├── scripts/
│   └── setup_udev.sh           # One-time udev rule setup for non-root dial access
├── tools/
│   ├── context_detector.py     # Sub-millisecond active window tracker (At-Spi / Win32 / macOS)
│   ├── donut_dial.py           # Filee-inspired animated Donut Radial HUD
│   ├── settings_gui.py         # Visual wheel customizer & preferences app
│   ├── tray_app.py             # System tray daemon (AppIndicator)
│   └── avrcp_dial.py           # Core dual-pairing Bluetooth daemon & hardware grabber
└── dial_app.py                 # Master unified desktop application
```

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

## Launching & Usage

### Option 1: Applications Menu (Recommended)
Press `Super` (Windows Key) and search for **Dial Control**, or find it in your App Drawer.
- Starts the background hardware listener.
- Places an icon in your top panel system tray.
- Opens the Settings & Customizer window.

### Option 2: Command Line
```bash
# Run the unified desktop application
./bin/dial-control

# Or run the settings customizer directly
python3 tools/settings_gui.py

# Or preview the animated Donut HUD without hardware
python3 tools/donut_dial.py
```

---

## One-Time Permissions Setup (Non-Root Execution)

To allow Dial Control to intercept dial clicks and emulate keys without requiring `sudo`:

```bash
bash scripts/setup_udev.sh
```

This installs `/etc/udev/rules.d/99-cmf-dial.rules` and grants your user account `uaccess` to the CMF Buds AVRCP input events.

---

## Customizing the Wheel

Open the customizer via the tray icon or by running:
```bash
python3 tools/settings_gui.py
```
- Reorder, add, or delete wheel slices.
- Assign custom hotkeys (`Ctrl+C`, `Shift+Alt+F`), media keys, or shell commands.
- Tweak dial outer radius, donut thickness, and haptic feedback.
- Changes saved to `config/profiles.json` take effect immediately on the live dial without restarting!
