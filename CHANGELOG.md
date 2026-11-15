# Changelog

Format based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow [SemVer](https://semver.org/).

## [Unreleased]

### Added
- Android: the desktop React UI now runs in WebViews (settings screen and wheel) through a Tauri-compatible shim, so profiles are edited the same way as on desktop; phone "system" slices (back, home, screenshot, torch, DND, swipes…).
- MIT license, contributing guide, security policy and refreshed documentation.

### Changed
- Documentation restructured: `docs/ARCHITECTURE.md`, `docs/PLATFORMS.md`, rewritten development guide; the RFCOMM protocol notes are marked as archived research.

### Removed
- Unused `config/profiles.json` (the live config is stored in the OS app-config directory).

## [1.0.0]

- Radial HUD with context-aware profiles, scroll mode, volume/brightness gauges, custom actions and macros.
- Linux, Windows and macOS backends; Android app started (work in progress, not released).
- Normal earbud behaviour by default; double press opens the wheel.
