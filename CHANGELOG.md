# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
- Project website (`website/`, Astro): the app's own wheel flies out of the hero in 3D, pins in front of a step-by-step walkthrough (it turns, clicks into the volume gauge, changes it, and so on) with leader lines drawn as you scroll, and the Settings window and keyboard get the same 3D-meets-2D treatment, the leader lines follow the clean-diagram rules (45 degree runs, never crossing, tested), and small interactions (headings that rise in word by word, an "Ask your AI" pill that copies a ready-made chatbot prompt (also served as `/llms.txt`), a leaning Settings window, keys that light as you press them, a progress spine) answer the pointer, and each walkthrough step shows its label before the dial acts; an OS-aware download section, the keyboard-only guide and structured data for search. Deployed to GitHub Pages by `.github/workflows/pages.yml`.
- `DonutRadialHUD` accepts an `inline` prop so the same component can render in normal flow outside the app.

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

### Removed
- Settings that did nothing: *Earbud audio feedback*, *Start resident in tray* (the window already hides to the tray when closed) and the slice *Telemetry hint* field. The unused `sensitivity`, `auto_close_seconds` and `desc` fields are dropped from saved configs on load, and the *Shortcut & Hardware* tab is now just *Shortcut*.
- Unused backend commands (`show_hud`, `toggle_hud`, `show_settings` and `set_system_brightness` as webview commands).

### Security
- udev setup no longer makes `/dev/uinput` or the dial's input node world-writable.
- Single-instance socket moved to the per-user runtime directory with `0600` permissions; IPC reads are bounded.
- Strict Content-Security-Policy; removed the remote Google Fonts request so the app makes no network calls.
- Updated `tokio` (RUSTSEC-2026-0384) and `rustix`.
- Release workflows use least-privilege tokens; Dependabot enabled.

### Changed
- The app now bundles the open-licensed Doto typeface (SIL OFL) instead of third-party fonts; the CSS class is `font-dot`. The Settings slice list header reads "Slices".
- Removed third-party branding from the app UI: themes are now *Signal Red* / *Amber* / *Pure Titanium* (saved configs are migrated); added an "independent project, no affiliation" notice.

### Not included
- The Android app (`android/`) is a work in progress and is not part of this release.
