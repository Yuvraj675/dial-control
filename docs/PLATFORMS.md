# Platform Implementation Notes

Deep-dive reference for each backend. For the overview see the [README](../README.md); for contributor rules see [CLAUDE.md](../CLAUDE.md).

## Windows

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

## macOS

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

## Android

A native Kotlin app (framework APIs only, minSdk 26) with the same gesture model: turn = volume, click = play/pause, double press = open the wheel, triple press = previous track; with the wheel open, turn moves the highlight and click runs the slice.

- **UI**: the desktop React bundle (`npm run build` → `dist/`) is copied into the APK assets by Gradle (`copyWebUi`, so build the frontend first) and runs in WebViews: the wheel overlay and a landscape `SettingsActivity`. `assets/shim/shim.js` implements `window.__TAURI_INTERNALS__` and sets `__DIAL_PLATFORM__ = "android"`, so `invoke`/events land in Kotlin (`web/Commands.kt`, `web/WebUi.kt`). Profiles are edited with the same editor as on desktop, limited to phone-capable slice types.
- **Dial input**: an own `MediaSession` looping silence (no audio focus, so your music keeps playing) receives the earbuds' Play/Pause/Next/Previous and is re-claimed whenever another app starts playing. Turns are `STREAM_MUSIC` index changes; while the wheel is open the index is locked with headroom and each change is one detent.
- **Accessibility service**: wheel overlay window, `dispatchGesture` swipes for scroll mode, foreground-app detection, global actions (back/home/recents/notifications/screenshot/lock).
- **Phone slices**: media, scroll/volume/brightness modes, swipes, global actions, launch app, torch, mute, Do Not Disturb. Desktop hotkeys cannot be injected without root.
- **Setup** (main-screen checklist): Accessibility service, notification access (to control the real player), modify system settings (brightness), optional DND access.
- **Known limits**: the system volume panel shows for dial turns; a click right after another app starts playing can still reach that app.
- **Build**: `npm run build` in the repo root, then `cd android && ./gradlew testDebugUnitTest assembleRelease` (Android SDK 36, **JDK 17+ with `javac`**). CI APKs use a throwaway debug key.
