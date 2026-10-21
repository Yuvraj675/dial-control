# Dial Control — Contributor & AI Agent Guidelines

Welcome to **Dial Control**. This document serves as the single source of truth and operating handbook for all developers and AI agents working on this codebase.

---

## 1. Core Philosophy & Primary Directive

### 🎯 Standalone Cross-Platform Product
The goal of this project is to build and ship a **standalone desktop application** that can be installed and run out-of-the-box by **anyone on Linux, macOS, and Windows** with full feature parity.

### 🌐 Cross-Platform First, Edge-Case Fallbacks Second
- **Do NOT build features solely for a specific laptop or desktop environment** (e.g., GNOME on Wayland).
- **Architecture Flow**:
  1. **Define Universal Core**: Design clean, platform-agnostic abstractions (Rust traits/modules).
  2. **Implement Platform Backends**: Build standard implementations for Windows (Win32/WASAPI), macOS (CoreAudio/IOKit/Cocoa), and Linux (WirePlumber/Pulse/sysfs).
  3. **Add Desktop Environment Edge-Case Fallbacks**: Only after the universal base is in place, layer in graceful workarounds for environment quirks (e.g. GNOME Wayland sandboxing, Mutter D-Bus, AT-SPI) without breaking or polluting other platforms.

### 🔍 Research Before Probing (Internet First)
- **Do not guess or run trial-and-error shell commands** on the host machine to figure out how OS-level features work.
- **Use the internet**: Search the web, consult official OS APIs, and inspect how established cross-platform applications and Rust crates solve problems (e.g., `volumecontrol`, `brightness`, `x-win`, `active-win-pos-rs`, `enigo`, `winit`).
- When researching, verify best practices and compatibility across Windows, macOS, and Linux.

---

## 2. Repository Cleanliness & Bloat Prevention

- **Keep the repository clean and release-ready**:
  - The tracked git repository must contain **only** production code, configuration, assets, and documentation.
  - Do **NOT** commit compiled release binaries (`bin/dial-control`, `.backup` files, `backups/`, or `target/`).
  - Do **NOT** commit raw APKs, XAPKs, or multi-megabyte reverse-engineering dumps.
- **Reference & Prototype Storage (`temp/`)**:
  - Any exploratory Python scripts, historical PyQt5 prototypes, Bluetooth sniffers, or backup binaries must live in `temp/`.
  - The `temp/` folder is strictly `.gitignore`d and must **never** be checked into git.
- **Continuous Documentation**:
  - Keep `README.md` updated after every major task or feature change. It must always reflect the current build instructions, tech stack, and cross-platform status.

---

## 3. Git Commit Rules

- **Author Identity**:
  Always commit using the user's author identity:
  ```bash
  git commit --author="Yuvraj675 <yuviyg675@gmail.com>" -m "<descriptive message>"
  ```
- **NO Co-Authors**:
  **NEVER** add `Co-authored-by` tags or credit other entities in commit messages.
- **Do NOT Push**:
  Never run `git push`. The user will review and push changes to remote when ready.

---

## 4. Architecture & Technology Stack

```
dial-control/
├── assets/                 # App icons and graphics
├── config/                 # profiles.json (wheel slices & context rules)
├── docs/                   # Architecture & protocol specifications
├── public/                 # Fonts (NType82, Ndot57) & web assets
├── scripts/                # Platform helper scripts (e.g. udev rules)
├── src/                    # Tauri v2 Frontend (React 18 + TS + Tailwind)
│   ├── components/
│   │   ├── hud/            # DonutRadialHUD (Framer Motion circular wheel)
│   │   └── settings/       # SettingsWindow (Slice editor & profile manager)
│   ├── data/               # defaultProfiles.ts
│   └── types/              # profile.ts
├── src-tauri/              # Tauri v2 Backend (Rust)
│   ├── Cargo.toml          # Rust dependencies with conditional platform targets
│   └── src/
│       ├── lib.rs          # App bootstrap, Tauri commands & tray manager
│       └── platform/       # Modular OS Abstractions
│           ├── mod.rs      # OS platform switcher & factory
│           ├── traits.rs   # Unified traits (Audio, Brightness, Media, Input, Dial, Context)
│           ├── linux/      # Linux backend (WirePlumber, Mutter, uinput, BlueZ, evdev)
│           ├── windows/    # Windows backend & stubs (WASAPI, Win32, RawInput)
│           └── macos/      # macOS backend & stubs (CoreAudio, IOKit, Cocoa)
└── temp/                   # [GITIGNORED] Legacy python prototypes & backups
```

### Subsystems Breakdown

| Subsystem | Windows | macOS | Linux Base | Linux Edge Case (GNOME Wayland) |
|---|---|---|---|---|
| **Volume Control** | WASAPI (`IAudioEndpointVolume`) | CoreAudio (`kAudioHardwareServiceDeviceProperty_VirtualMasterVolume`) | WirePlumber (`wpctl`) / PulseAudio (`pactl` / `libpulse`) | Same as Linux Base |
| **Brightness Control** | WMI (`WmiMonitorBrightness`) / Win32 Monitor API | DisplayServices / IOKit | `/sys/class/backlight` / `systemd-logind` | Mutter D-Bus `SetBacklight` fallback |
| **Active Window Context** | Win32 `GetForegroundWindow` + `GetWindowTextW` | Cocoa `NSWorkspace.frontmostApplication` | X11 `_NET_ACTIVE_WINDOW` / `wlr-foreign-toplevel` | GNOME FocusedWindow D-Bus + native multi-compositor fallbacks |
| **Key Simulation** | Win32 `SendInput` / `enigo` | `CGEventPost` / `enigo` | `enigo` / `/dev/uinput` | `/dev/uinput` via `evdev` |
| **Dial Interception** | RawInput / Bluetooth AVRCP | CoreBluetooth / IOHIDManager | BlueZ `MediaTransport1` + `evdev` grabber | Same as Linux Base |

---

## 5. Development & Build Workflow

### ⚠️ CRITICAL: Production Frontend Bundling
- **NEVER use raw `cargo build --release` to generate production binaries.**
  - **Why**: Raw `cargo build` does NOT bundle frontend assets. It leaves the binary expecting an active Vite dev server (`http://localhost:1420`), causing windows like Settings or HUD to fail with `"Could not connect to localhost"`.
  - **Mandatory Production Build Command**:
    ```bash
    # 1. Build using Tauri CLI (compiles frontend + inlines assets into binary):
    npx @tauri-apps/cli build --no-bundle
    # (or `pnpm tauri build --no-bundle`)

    # 2. Update local launcher binary:
    cp src-tauri/target/release/dial-control bin/dial-control
    ```
  - This guarantees:
    1. `beforeBuildCommand` (`npm run build`) runs `tsc && vite build` into `dist/`.
    2. Tauri embeds `frontendDist: "../dist"` directly inside the executable.
    3. The application runs completely standalone without any web server or localhost dependency.

### 🪟 Window Lifecycle & Geometry Guidelines
- **Hide on Close (Prevent Window Destroy)**:
  - Desktop utility windows (e.g. `settings`, `hud`) opened from the tray must **never be destroyed** when the user clicks the `X` close button.
  - Intercept `tauri::WindowEvent::CloseRequested` in `.on_window_event()` on the Tauri Builder:
    ```rust
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.label() == "settings" || window.label() == "hud" {
            let _ = window.hide();
            api.prevent_close();
        }
    }
    ```
  - In `show_settings`, always call `window.show()`, `window.unminimize()`, and `window.set_focus()`. Provide a fallback to dynamically recreate the window if it was ever destroyed.
- **Window Dimensions & Layout Hygiene**:
  - The settings window displays a 3-column layout (Profiles list + Wheel slices + Slice inspector) plus a left sidebar. Configure initial size to `1080x680` with `minWidth: 900`, `minHeight: 550`.
  - Column containers in `SettingsWindow.tsx` must use `shrink-0` on sidebar/navigation columns and `overflow-x-auto` on the content wrapper so that right-hand controls are never cut off on scaled laptop screens (125%-150% DPI).

### 🎨 Taskbar & Window Icon Synchronization
- **Desktop Entry & WM Class Alignment**:
  - GNOME/Wayland taskbars match running windows to desktop icons using the application's WM_CLASS or App ID (`StartupWMClass=dial-control`).
  - Keep `~/.local/share/applications/dial-control.desktop` and icons under `~/.local/share/icons/hicolor/` synchronized with `assets/dial-control.png`.
  - In `src-tauri/icons/icon.png`, ensure PNGs retain a 32-bit RGBA channel (`RGBA`) to prevent image decoding panics in Tauri's icon compiler.

### ⌨️ Global Shortcuts & Wayland IPC
- **Wayland Security & IPC Socket**:
  - On Linux Wayland (GNOME/KDE/Sway), security policies prevent client windows from intercepting unfocused global shortcuts.
  - Dial Control solves this with high-speed local IPC: `main.rs` intercepts `--toggle` and writes to `/tmp/dial-control.sock` in < 2ms, triggering the HUD immediately.
  - GNOME compositor keybindings (`org.gnome.settings-daemon.plugins.media-keys.custom-keybinding`) are synchronized automatically whenever shortcuts are recorded in Settings.
- **Interactive Shortcut Recording**:
  - The UI uses hardware `keydown`/`keyup` event capture rather than manual text input, detecting modifiers (`Super`, `Ctrl`, `Alt`, `Shift`) and normal keys. Presets like `Super+Shift+D` and `Super+Backslash` are provided out-of-the-box.

### Standard Step-by-Step Cycle
1. **Research First**: Search web/docs for established cross-platform patterns before writing platform code.
2. **Modularize Platforms**: Use Rust conditional compilation (`#[cfg(target_os = "...")]`) or trait abstractions.
3. **Frontend Verification**: Run `npm run build` to catch TypeScript/JSX compilation issues early.
4. **Backend Verification**: Run `cargo check --manifest-path src-tauri/Cargo.toml` to verify Rust code.
5. **Production Release**: Run `npx @tauri-apps/cli build --no-bundle` and copy binary to `bin/dial-control`.
6. **Update `README.md`**: Ensure documentation matches actual code state.
7. **Commit**: Use specified author name (`Yuvraj675 <yuviyg675@gmail.com>`) with NO co-authors.
