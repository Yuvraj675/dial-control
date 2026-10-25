# CMF Buds Pro 2 - Smart Dial Control System

Turn your **CMF Buds Pro 2** smart dial case into a context-aware, beautifully animated radial controller for **Linux, Windows, and macOS** - inspired by **[KnifeLemon/Filee](https://github.com/KnifeLemon/Filee)** and the Nothing OS aesthetic.

- **100% Dual-Pairing Preserved**: Keeps your phone connection and the Nothing X mobile application active simultaneously.
- **Scroll Mode (Mouse Wheel Emulation)**: Rotate the dial to scroll long web pages, documents, code, or video feeds with zero window focus loss and native OS notifications.
- **Stateful Volume Locking**: Keeps your system audio playback volume locked at its baseline while browsing menus, scrolling, or adjusting brightness, with hardware-level echo suppression.
- **Live Display Brightness**: Accurate live brightness reading and stepping across platforms (including Intel/Samsung OLED laptops via GNOME Mutter D-Bus, external monitors via DDC/CI on Windows, and DisplayServices on macOS).
- **Single Instance**: Launching Dial Control again (application menu, autostart) brings the running instance's settings window forward instead of starting a second copy and tray icon.
- **Direct Launch & System Tray Docking**: Launches directly into the Settings & Customizer window on startup. Closing the window minimizes seamlessly to the system tray background daemon.
- **Nothing OS Typography**: Fully styled with authentic Nothing OS dot-matrix typography (`Ndot57`) across all HUD dials, settings panels, slice inspectors, and configuration cards.
- **Custom Actions & Macros**: Any slice can press a hotkey, open an app (picked from your installed apps), file, folder or URL, run a shell command, type a text snippet, or run a **macro**: an ordered sequence of those steps plus waits and media keys (e.g. open VS Code → wait 800 ms → Ctrl+Shift+P → type "format"). A *Test run* button tries the slice after a 3 s countdown. On Linux, typed text assumes a US keyboard layout (non-ASCII text needs `wtype` or `xdotool`).
- **Custom Dials Creator**: Create custom dial profiles from scratch with custom shortcut triggers, import actions across any dial, and configure dynamic window title/class pattern matching.
- **Global Activation Shortcut**: Full keyboard-only radial control for users without CMF Buds hardware. Equipped with a conflict-free default (`Super+Shift+D`) and an in-app interactive shortcut rebinder.
- **Custom Nothing App Icon**: Distinctive dot-matrix circular emblem integrated into taskbars, window decorations, system notifications, and system tray.
- **Normal Earbud Controls by Default**: While the HUD is closed the dial works as usual: click = play/pause, turn = volume, triple press = previous track.
- **Double-Press the Dial / Press Shortcut**: An animated, glowing Donut Radial HUD pops up on screen with options tailored to your currently focused application. In the **Nothing X** app (Controls → Dial) the dial's double press must be set to *Next track* and triple press to *Previous track*: the earbuds send those commands, and Dial Control reads them as the gestures.
- **Scroll Mode Indicator**: While the dial acts as a scroll wheel, the tray icon turns red (with an *Exit Scroll Mode* menu item), GNOME shows a red SCROLL pill in the top bar, and a short tone plays in your buds on entry and exit.
- **Home Dial Option**: Every dial menu features a `HOME` option that opens the interactive Home Dial switcher where you can manually choose between any dial menu (System, Media, Code, Files, Browser, Design, or custom profiles).
- **Rotate the Dial / Arrow Keys**: Smoothly cycles options with audio feedback while your media volume remains locked.
- **Click / Enter**: Executes the highlighted action, with or without turning first (such as `Copy`/`Paste` in Files, `Seek`/`Fullscreen` in YouTube, `Format` in VS Code, or opens the Volume/Brightness/Scroll mode). Double or triple press closes the HUD without running anything; in scroll mode a click or a double press exits.
- **System Tray Daemon**: Runs quietly in your system tray / top panel with clean, text-based controls, radial toggles, and settings customizer.
- **Built with Tauri v2 + Rust + React**: Native performance, minimal memory footprint, and cross-platform architecture.

---

## Architecture Overview

```
dial-control/
├── assets/                 # Desktop & system tray icons (PNG, SVG, ICO)
├── config/                 # Declarative profiles.json configuration
├── docs/                   # Protocol & architectural specifications
├── public/                 # Typography (NType82, Ndot57) & assets
├── resources/              # GNOME Shell extension & OS assets
├── scripts/                # Packaging, udev rules & build scripts
├── src/                    # Frontend UI (React 19, TypeScript, Tailwind CSS)
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
│           ├── common.rs   # Shared profile matcher & context event
│           ├── linux/      # WirePlumber, Mutter, uinput, BlueZ, evdev & D-Bus Context
│           ├── windows/    # Core Audio COM, dxva2 DDC/CI, WMI/CIM, Win32 hooks
│           └── macos/      # CoreAudio, DisplayServices, CoreGraphics Event Tap & Cocoa
└── dist-packages/          # Pre-assembled Linux & Windows installation packages
```

---

## Cross-Platform Roadmap & Status

| Platform | Dial / AVRCP Interception | Volume Control | Brightness Control | Context Detection | Status |
|---|---|---|---|---|---|
| **Linux (X11 / Wayland)** | BlueZ D-Bus + `evdev` exclusive grab | WirePlumber (`wpctl`) / PulseAudio | `/sys/class/backlight` + GNOME Mutter D-Bus | GNOME FocusedWindow D-Bus + Hyprland/Sway/KDE/X11 | **Fully Functional** |
| **Windows (10 / 11)** | Low-Level Hook (`WH_KEYBOARD_LL`) | Windows Core Audio COM (`IAudioEndpointVolume`) | Dual-layer: `dxva2.dll` (DDC/CI) + WMI/CIM | Win32 `GetForegroundWindow` + Process Image & Class Name | **Fully Functional** |
| **macOS (Sonoma / Sequoia / Apple Silicon & Intel)** | CoreGraphics Event Tap (`kCGHIDEventTap`) | CoreAudio (`VirtualMasterVolume`) | DisplayServices hardware backlight | Cocoa `NSWorkspace` + AppleScript title query | **Fully Functional** |
| **Android (8.0+)** | Own `MediaSession` (clicks) + `STREAM_MUSIC` volume watcher (turns) | `AudioManager` | `Settings.System.SCREEN_BRIGHTNESS` | Accessibility `TYPE_WINDOW_STATE_CHANGED` package name | **Early (untested on device)** |

---

## Context-Aware Profiles & Home Dial

The dial automatically detects your active foreground application and adapts its options in real-time. In addition, you can rotate to **HOME** in any dial menu to manually open the **Home Dial** launcher and choose between different profiles:

| Foreground App | Profile | Slices on the Wheel |
|---|---|---|
| **Files / Nautilus / Dolphin** | **Files & Folders** | Copy (`Ctrl+C`), Paste (`Ctrl+V`), Trash (`Delete`), New Folder (`Ctrl+Shift+N`), Rename (`F2`), Terminal, Home Dial |
| **YouTube / Firefox / Chrome / Zen** | **Video & Media** | Play/Pause, Seek +10s (`Right`), Seek -10s (`Left`), Fullscreen (`F`), Mute (`M`), Volume Gauge, Home Dial |
| **VS Code / Cursor / Zed** | **Code & Editor** | Save All (`Ctrl+S`), Format (`Shift+Alt+F`), Find in Files (`Ctrl+Shift+F`), Comment (`Ctrl+/`), Terminal (`Ctrl+\``), Git Diff, Home Dial |
| **Desktop / General Apps** | **System Controls** | Volume Adjust, Brightness Adjust, Play/Pause, Next Track, Prev Track, Mute Toggle, Home Dial |
| **Any Screen (via Home)** | **Home Dial Switcher** | System, Media, Code, Files, Browser, Design (plus any custom user profiles) |

---

## Windows Implementation Details

The Windows port runs with native Win32 and COM integrations designed for maximum responsiveness, security, and hardware compatibility:

- **Earbuds Dial Case Interception**: Uses dual-channel Windows Core Audio Endpoint Volume Notifications (`IAudioEndpointVolumeCallback`) for Bluetooth AVRCP rotary turns with a floating headroom model and dual-layer echo cancellation, plus a Win32 Low-Level Keyboard Hook (`WH_KEYBOARD_LL`) for center clicks and gestures. When the HUD is closed, a click is forwarded to the real player as play/pause, a double press (AVRCP Next) opens the radial HUD and a triple press (Previous) goes back a track. While open, rotating navigates slices, clicking confirms the highlighted slice, and a double or triple press dismisses the HUD without skipping tracks in background players.
- **Own Media Session**: Windows sends Bluetooth play/pause/next/previous to the current media session, so while YouTube or a browser holds it the click would pause the video and never reach the app. Dial Control registers its own silent media session (with automatic re-assertion when another app takes over) so dial presses are delivered to it. Dial clicks with the HUD closed are forwarded to the real player (`TryTogglePlayPauseAsync`).
- **Raw Input Click Detection**: Bluetooth AVRCP sends discrete Play / Pause for the dial click, which Windows delivers as `WM_APPCOMMAND` without any virtual key, so a keyboard hook alone never sees it. A Raw Input listener on the shared HID Consumer Control collection (`RIDEV_INPUTSINK`) detects those presses; it shares one debounced click handler with the hook. Note: Raw Input can observe but not swallow, so a discrete Play/Pause click may still toggle the active media player on Windows.
- **Windows 11 Volume Flyout Suppression**: Features an out-of-context WinEvent hook (`SetWinEventHook`) that automatically suppresses the Windows 11 volume bar pill (`XamlExplorerHostIslandWindow`) during dial turns so the OS volume popup never obscures the HUD.
- **Dynamic Audio Endpoint Rebinding**: Continuously tracks the active default render audio endpoint, automatically rebinding the Core Audio volume callback when CMF Buds Pro 2 are connected or reconnected.
- **Hardware Master Volume (Zero Desynchronization)**: Directly interfaces with the Windows Core Audio COM API (`IAudioEndpointVolume` and `IMMDeviceEnumerator` via `ole32.dll`). It reads the exact system volume level on startup and sets the hardware master scalar directly without keypress simulation.
- **Dual-Layer Display Brightness**:
  - **Internal Laptop Screens**: Dual-execution PowerShell script invoking both `(Get-WmiObject -Namespace root/wmi -ClassName WmiMonitorBrightnessMethods).WmiSetBrightness(...)` and `Invoke-CimMethod` with 150ms debouncing and `CREATE_NO_WINDOW`.
  - **External Desktop Monitors**: Direct Win32 High-Level Monitor Configuration API (`dxva2.dll` / `SetMonitorBrightness`) via Direct3D/DDC/CI.
- **Context & Desktop Detection**: Distinguishes Windows desktop components (`Progman`, `WorkerW`, `Shell_TrayWnd`) from actual File Explorer folder windows (`CabinetWClass`), ensuring the desktop routes to System Controls instead of Files.
- **Input Tagging**: Stamped synthetic media inputs with `SYNTHETIC_INPUT_TAG` (`0x4449414C`) to prevent the low-level hook from blocking its own emitted keystrokes.
- **Overlay Window Presentation**: HUD window uses `"skipTaskbar": true` and `"shadow": false` to eliminate rectangular borders/shadows, and auto-dismisses smoothly when focus is lost (`Focused(false)` / clicking outside).
- **Interactive Terminal Launcher**: Uses progressive PATH detection (`where wt.exe ... && start wt.exe || where powershell.exe ... || start cmd.exe`) to launch Windows Terminal, PowerShell, or Command Prompt without console suppression.
- **Security & Least-Privilege**: Runs completely without Administrator privileges in user scope (`%LOCALAPPDATA%\Programs\dial-control`). Respects Windows UIPI (User Interface Privilege Isolation) so hotkeys can be delivered to user web browsers and applications.

---

## macOS Implementation Details

The macOS backend uses native Apple frameworks (`CoreGraphics`, `CoreAudio`, `CoreFoundation`, and `DisplayServices`) for high performance and seamless integration with macOS Sonoma, Sequoia, and earlier releases:

- **Hardware-Level Event Consumption (`CGEventTap`)**: Intercepts CMF Buds Pro 2 rotary events via an event tap at `kCGHIDEventTap` listening to `NX_SYSDEFINED` events (`NX_SUBTYPE_AUX_CONTROL_BUTTONS`). When volume locking or scroll mode is active, the callback returns `null_mut()`. This drops the event at the CoreGraphics driver layer, completely preventing macOS from changing system volume or showing the volume bezel overlay, eliminating feedback loops.
- **AVRCP Absolute Volume Rotation**: A CoreAudio property listener on the default output device (re-bound when the output changes) detects dial turns that macOS applies directly to the Bluetooth device volume, snapping back to a headroom baseline while the HUD is open.
- **Focus Hand-Back & Hotkeys**: When the HUD hides, the previously active app is re-activated so hotkeys reach it; built-in slices carry dedicated macOS shortcuts (Command-based, e.g. Redo = Cmd+Shift+Z, Browser Back = Cmd+[), while Windows and Linux keep their own. Custom shortcuts without a Mac form are translated automatically (Ctrl -> Command).
- **Earbud Click Routing (Now Playing)**: Bluetooth earbud presses go to the macOS Now Playing app, not to the keyboard event path, so Dial Control registers itself as the Now Playing app (`MPRemoteCommandCenter` plus a looping silent player, re-claimed whenever the HUD opens or closes, the dial turns or the foreground app changes). With the HUD closed a click is forwarded to the real player as play/pause and a double press (Next) opens the HUD; media slices are forwarded to the real player (AppleScript for Spotify and Music). Dial Control runs as a menu-bar app (no Dock icon) so the HUD can appear over full-screen apps. The macOS volume overlay that appears while turning the dial is drawn by the system for Bluetooth absolute volume and cannot be hidden by apps. Diagnostics: `$TMPDIR/dial-control/dial-control.log`.
- **Center-Click Interception**: Center clicks (`NX_KEYTYPE_PLAY`) are intercepted with a 250ms debounce. The event is dropped and, with the HUD closed, re-sent to the player as play/pause, so Apple Music and Spotify never toggle while navigating the HUD.
- **Scroll Mode (Mouse Wheel Emulation)**: Emulates native CoreGraphics scroll wheel events via `CGEventCreateScrollWheelEvent2` with `kCGScrollEventUnitLine`. The HUD window automatically hides after 850ms, returning focus cleanly to the target browser or editor.
- **Native Master Volume**: Direct C-FFI queries and updates to CoreAudio's `VirtualMasterVolume` and scalar properties on `kAudioObjectSystemObject`, with an `osascript` fallback.
- **Display Brightness & Stepping**: Interacts with built-in MacBook displays directly via `DisplayServicesGetBrightness` and `DisplayServicesSetBrightness` for accurate live percentage readings and smooth 5% stepping, with a `brightness` CLI fallback.
- **Native Auxiliary Media Keys**: Simulates system-defined multimedia keys (`NX_KEYTYPE_PLAY`, `NX_KEYTYPE_NEXT`, `NX_KEYTYPE_PREVIOUS`) via AppKit `[NSEvent otherEventWithType:14 ...]` bridged directly to `CGEventPost(kCGHIDEventTap)`, controlling any active media player system-wide (Apple Music, Spotify, YouTube in Safari/Chrome, VLC) with an AppleScript fallback.
- **Active Application Context**: Monitors foreground applications in real time using `NSWorkspace` and AppleScript to extract active application bundle identifiers (e.g. `com.google.Chrome`, `com.microsoft.VSCode`, `com.spotify.client`) and window titles, matching them to custom dial profiles.
- **Notification Center Integration**: Dispatches native macOS Notification Center alerts via `osascript` on scroll mode entry and exit.
- **Permissions**: Requires a single user toggle under *System Settings $\to$ Privacy & Security $\to$ Accessibility* for global event interception and scroll emulation, matching standard macOS utilities like Alfred and Raycast.

---

## Android App (`android/`)

A native Kotlin app (no third-party dependencies) with the same gesture model: turn = volume, click = play/pause, double press = open the wheel, triple press = previous track; with the wheel open, turn moves the highlight and click runs the slice.

- **Dial input**: an own `MediaSession` looping silence (no audio focus, so your music keeps playing) receives the earbuds' Play/Pause/Next/Previous, and is re-claimed whenever another app starts playing. Turns are `STREAM_MUSIC` index changes; while the wheel is open the index is locked with headroom and each change is one detent.
- **Wheel overlay, scroll mode, per-app profiles and system actions** run in an Accessibility Service (overlay window, `dispatchGesture` swipes, foreground package, back/home/recents/notifications/screenshot/lock).
- **Phone slices**: media, scroll / volume / brightness modes, swipes (next Short / post), global actions, launch app, torch, mute, Do Not Disturb. Desktop hotkeys cannot be injected on Android without root.
- **Setup** (main screen checklist): Accessibility service, notification access (to play/pause the real player), modify system settings (brightness), optional DND access. The main screen shows a live log of every dial signal (`adb logcat -s DialControl`).
- **Known limits**: the system volume panel appears for dial turns and cannot be hidden; a click right after another app starts playing can still reach that app; there is no in-app profile editor yet.
- **Build**: `cd android && ./gradlew testDebugUnitTest assembleRelease` (Android SDK 36, JDK 17+). The APK is also built by the manually triggered **Build Android APK** workflow (Actions tab -> Run workflow). CI APKs are signed with a throwaway debug key, so uninstall before installing a newer one.

---

## Tiling Window Managers (Hyprland, Sway, i3)

By default, tiling compositors place all new windows into the tiling layout. To have the radial HUD display as a floating, centered overlay with zero borders, add the following configuration:

### Hyprland (`~/.config/hypr/hyprland.conf`)
```ini
# Dial Control HUD - Floating Radial Overlay
windowrulev2 = float, class:^(dial-control)$, title:^(Dial Control HUD)$
windowrulev2 = center, class:^(dial-control)$, title:^(Dial Control HUD)$
windowrulev2 = noborder, class:^(dial-control)$, title:^(Dial Control HUD)$
windowrulev2 = noshadow, class:^(dial-control)$, title:^(Dial Control HUD)$
windowrulev2 = pin, class:^(dial-control)$, title:^(Dial Control HUD)$
```

### Sway / i3 (`~/.config/sway/config` or `~/.config/i3/config`)
```ini
for_window [app_id="dial-control" title="Dial Control HUD"] floating enable, border none
```

---

## Pre-Built Distribution Packages

Dial Control provides native, pre-assembled packages for all major Linux distributions in `dist-packages/` and Windows releases via GitHub Actions:

### Linux Packages

| Distribution | Package Format | Installation Command |
|---|---|---|
| **Debian / Ubuntu 24.04+ / Pop!_OS / Mint** | `.deb` | `sudo dpkg -i dist-packages/dial-control_1.0.0_amd64.deb` |
| **Fedora 40/41 / Nobara / RHEL / openSUSE** | `.rpm` | `sudo dnf install ./dist-packages/dial-control-1.0.0-1.x86_64.rpm` |
| **Arch Linux / Manjaro / EndeavourOS** | `.pkg.tar.zst` | `sudo pacman -U ./dist-packages/dial-control-1.0.0-1-x86_64.pkg.tar.zst` |
| **Any Linux (Portable)** | `.AppImage` | `chmod +x dist-packages/Dial-Control-1.0.0-amd64.AppImage && ./dist-packages/Dial-Control-1.0.0-amd64.AppImage` |

#### Bundled Inside the Linux Packages
- **Application Binary**: Installed to `/usr/bin/dial-control`.
- **Desktop Entry & Icon**: Registered under `/usr/share/applications/` and `/usr/share/icons/hicolor/512x512/apps/`.
- **Hardware Access Rules (`udev`)**: Installs `/usr/lib/udev/rules.d/99-dial-control.rules` giving the desktop user non-root access to `/dev/uinput` and CMF Buds Pro 2 rotary input events.
- **GNOME Shell Focused Window Extension**: Installs the `focused-window-dbus@flexagoon.com` extension system-wide to `/usr/share/gnome-shell/extensions/` to enable Wayland context detection.

### Windows Packages

Windows packages are compiled via **GitHub Actions** (`.github/workflows/build-windows.yml`), started manually from the Actions tab (**Run workflow**):
- **`Dial Control_1.0.0_x64-setup.exe`**: Per-user NSIS Installer with automatic Start Menu and desktop integration.
- **`Dial Control_1.0.0_x64_en-US.msi`**: Windows Installer package with standard Windows Add/Remove Programs integration.

Download the latest installer from the GitHub repository's **Actions** tab (under workflow run artifacts) or **Releases**.

### macOS Packages

macOS packages are compiled via **GitHub Actions** (`.github/workflows/build-macos.yml`) on `macos-latest` runners, started manually from the Actions tab (**Run workflow**):
- **`Dial Control_1.0.0_aarch64.dmg`**: Native Apple Silicon installer (M1/M2/M3/M4) with drag-to-Applications bundle.
- **`Dial Control.app`**: Standalone application bundle (zipped).

Download the latest `.dmg` or `.app` from the GitHub repository's **Actions** tab or **Releases**.

The app is ad-hoc signed (`bundle.macOS.signingIdentity: "-"`), not notarized with an Apple Developer ID, so Gatekeeper blocks it on first launch (an older unsigned build showed *"is damaged and can't be opened"*). Install it like this:
1. Open the `.dmg` and drag **Dial Control** into **Applications**.
2. Remove the download quarantine flag once:
   ```bash
   xattr -dr com.apple.quarantine "/Applications/Dial Control.app"
   ```
   (or right-click the app, choose **Open**, then confirm under *System Settings → Privacy & Security → Open Anyway*).

---

## Distribution Compatibility & System Requirements

### Compatibility Matrix

| Distribution / Environment | Status | Details & Requirements |
|---|---|---|
| **macOS Sonoma (14) & Sequoia (15+)** | **Supported** | Native `.dmg` / `.app` (Apple Silicon & Intel). Requires Accessibility permission. |
| **Windows 11 & Windows 10** | **Supported** | Native `.exe` / `.msi`. Pre-installed WebView2 runtime. |
| **Ubuntu 24.04 LTS & 24.10+** | **Supported** | Native `.deb` works out of the box. GNOME Shell 46+. |
| **Fedora 40, 41 & Nobara** | **Supported** | Native `.rpm` works out of the box. GNOME Shell 46/47+. |
| **Arch Linux / Manjaro / EndeavourOS** | **Supported** | Native `.pkg.tar.zst` works out of the box. |
| **openSUSE Tumbleweed** | **Supported** | Native `.rpm` works out of the box. |
| **KDE Plasma (Any Modern Distro)** | **Supported** | Natively queries active window via `kdotool`. Zero extensions needed. |
| **Hyprland / Sway (Wayland)** | **Supported** | Supported with floating window rules. |
| **X11 / XWayland** | **Supported** | Natively queries active window via `xdotool`. |
| **Ubuntu 22.04 LTS / Debian 12** | **Incompatible** | Requires `GLIBC >= 2.39`. See note below for older distros. |

### Technical Details & First-Time Setup Notes

1. **GLIBC Requirement (`GLIBC_2.39`)**:
   The release binaries are compiled on Fedora/Nobara with `GLIBC_2.39`. Modern Linux releases (Ubuntu 24.04+, Fedora 40+, Arch) satisfy this requirement.
2. **GNOME Wayland First-Time Session Restart**:
   After installing Dial Control on a **GNOME Wayland** system for the first time, log out and log back in once (or reboot) so GNOME Shell loads the newly placed system extension.
3. **Bluetooth Connection Order**:
   If your CMF Buds Pro 2 were already connected via Bluetooth before running the package installation, disconnect and reconnect the earbuds once over Bluetooth so `systemd-logind` attaches the new `uaccess` permissions to the input node.
4. **AppImage Considerations**:
   When using the standalone AppImage, either run `bash scripts/setup_udev.sh` once or add your user to the `input` group (`sudo usermod -aG input $USER`).

---

## Getting Started

### Prerequisites

- **Node.js** (v18+) and **pnpm** (or npm)
- **Rust** (stable toolchain)
- **Linux Packages** (if developing on Linux):
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

---

## Tests

```bash
cargo test --manifest-path src-tauri/Cargo.toml --lib   # dial logic, keymaps, profile matching (all OS paths)
npm test                                                # frontend context matching & profile config
```

The Windows and macOS dial decision logic (click state machine, volume echo filtering, floating headroom, aux-key decoding, HID consumer edge detection, key translation) lives in platform-agnostic modules, so it is exercised on any host.

## Development & Building

### Building on Linux

```bash
# 1. Install dependencies
pnpm install

# 2. Run in development mode (hot reloading UI + Rust backend)
pnpm tauri dev

# 3. Build standalone production binary
pnpm tauri build --no-bundle
# (or: npx @tauri-apps/cli build --no-bundle)

# 4. Copy to launcher bin/
cp src-tauri/target/release/dial-control bin/dial-control

# 5. Build all multi-distribution packages (.deb, .rpm, AppImage, Arch Linux)
bash scripts/build-packages.sh
```

### Building on Windows (Local Machine or VM)

On a Windows system with Node.js and Rust installed:

```powershell
# Run the automated build script (or double-click scripts\build-windows.bat):
.\scripts\build-windows.ps1
```

This compiles:
- `Dial Control_1.0.0_x64-setup.exe` (NSIS Installer)
- `Dial Control_1.0.0_x64_en-US.msi` (WiX MSI Installer)
under `src-tauri/target/release/bundle/`.

Alternatively, run the **GitHub Actions** workflow (`build-windows.yml`) manually from the Actions tab to compile the `.exe` and `.msi` installers in the cloud with zero local setup needed.

### Building on macOS (Local Machine or CI)

On a macOS system with Node.js and Rust installed:

```bash
# 1. Install dependencies
pnpm install

# 2. Build native macOS packages (.dmg and .app bundle)
npx @tauri-apps/cli build --bundles dmg,app
```

Bundles will be generated under `src-tauri/target/release/bundle/dmg/` and `src-tauri/target/release/bundle/macos/`.

Alternatively, run the **GitHub Actions** workflow (`build-macos.yml`) manually from the Actions tab to build the `.dmg` installer and `.app` bundle on GitHub's Apple Silicon runners.

---

## Customizing Profiles & Global Shortcuts

Profiles, wheel slices, and activation triggers can be customized via the **Settings Window** (which opens on launch or via the system tray menu) or by editing `config/profiles.json`:

- **Create Custom Dials**: Add dedicated radial profiles tailored to any application or workflow with custom icons and title/WM_CLASS window matching rules.
- **Import Slices**: Pick and reuse actions from any existing dial (e.g. VS Code shortcuts or media controls) directly into your custom dial with a single click.
- **Reorder & Edit Slices**: Adjust wheel slice order, customize labels, reassign key combinations (`Ctrl+C`, `Shift+Alt+F`), media keys, or arbitrary shell commands.
- **Interactive Shortcut Recorder & Wayland IPC**: Record any global activation shortcut directly from your keyboard without manual typing. On Linux Wayland, automatically binds with GNOME compositor keybindings and triggers the radial HUD in < 2ms via a high-speed local IPC socket.
- **Radial Geometry & Aesthetics**: Modify inner/outer radius, donut thickness, accent glow colors, and rotation animations.
- **Instant Hot-Reload**: Changes saved in Settings take effect immediately on the live radial dial without restarting the daemon.
