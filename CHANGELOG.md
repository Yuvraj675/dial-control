# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/).

## [Unreleased]

## [1.0.0] - 2026-11-16

First release.

### Added
- Radial HUD with context-aware profiles, a Home switcher and custom profiles.
- Normal earbud behaviour by default (turn = volume, click = play/pause, triple press = previous); double press opens the wheel.
- Scroll mode with tray, GNOME top-bar and audio indicators; volume and brightness gauges.
- Custom slices: hotkeys, launch app/file/URL, shell command, type text and multi-step macros.
- Linux (BlueZ/evdev), Windows (Core Audio, hooks, own SMTC session) and macOS (event tap, own Now Playing session) backends; `.deb`, `.rpm`, `.pkg.tar.zst`, `.AppImage`, `.exe`, `.msi` and `.dmg` packaging.
- MIT license, contributing guide, security policy and architecture/platform documentation.
- Tag-driven release workflow and `scripts/bump-version.sh`.
- One-line installers (`install.sh` for Linux/macOS, `install.ps1` for Windows) that detect the OS and distribution, verify SHA-256 checksums and install the latest release.
- Keyboard controls documented: the whole wheel works from a global shortcut, arrow keys and the mouse wheel, no earbuds required.

### Security
- udev setup no longer makes `/dev/uinput` or the dial's input node world-writable.
- Single-instance socket moved to the per-user runtime directory with `0600` permissions; IPC reads are bounded.
- Strict Content-Security-Policy; removed the remote Google Fonts request so the app makes no network calls.
- Updated `tokio` (RUSTSEC-2026-0384) and `rustix`.
- Release workflows use least-privilege tokens; Dependabot enabled.

### Changed
- Removed third-party branding from the app UI: themes are now *Signal Red* / *Amber* / *Pure Titanium* (saved configs are migrated); added an "independent project, no affiliation" notice.

### Not included
- The Android app (`android/`) is a work in progress and is not part of this release.
