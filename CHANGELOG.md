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

### Not included
- The Android app (`android/`) is a work in progress and is not part of this release.
