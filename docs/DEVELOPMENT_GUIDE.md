# Development Guide

## 1. Setup

Prerequisites and system packages are listed in the [README](../README.md#build-from-source).

```bash
pnpm install
pnpm tauri dev          # Vite dev server on :1420 + Rust backend
```

No earbuds? Use the global shortcut (default `Super+Shift+D`) or the arrow keys / Enter while the HUD has focus. A browser-only preview (`pnpm dev`) renders the UI without the Rust backend.

## 2. Project map

| Path | Purpose |
|---|---|
| `src/components/hud` | Donut wheel (Framer Motion) |
| `src/components/settings` | Profile/slice editor, `ActionEditors.tsx` for launch/macro/test-run |
| `src/data/defaultProfiles.ts` | Built-in profiles and config migration |
| `src-tauri/src/lib.rs` | App state, Tauri commands, tray, single-instance IPC |
| `src-tauri/src/actions.rs` | Slice → steps executor |
| `src-tauri/src/platform/` | Traits, pure logic, and the three OS backends |
| `android/` | Native Kotlin app + WebView shim (see [PLATFORMS.md](PLATFORMS.md#android)) |
| `resources/gnome-extension` | Focused-window D-Bus extension |
| `scripts/` | Packaging, udev, Windows build |

## 3. Tests and checks

See [CONTRIBUTING.md](../CONTRIBUTING.md#checks) for the full list. When fixing a backend bug, extract the decision into `dial_logic.rs`, `keymap.rs` or `common.rs` and add a regression test there.

## 4. Adding a built-in slice

1. Add the slice to the profile in `src/data/defaultProfiles.ts` (`keys` for Windows/Linux, optional `keys_mac` written for macOS).
2. Use an icon from `src/utils/iconMap.tsx`.
3. Run `npm test`: the config and keymap tests verify every key token maps to a real keycode.

New *action types* need: the TypeScript type, an `actions::steps_for` arm, the editor in `ActionEditors.tsx`, and a test.

## 5. Production build

```bash
npx @tauri-apps/cli build --no-bundle
install -m 755 src-tauri/target/release/dial-control bin/dial-control
./scripts/build-packages.sh
```

A plain `cargo build --release` skips the frontend and produces a binary that points at `localhost:1420`.

## 6. Diagnostics

- Windows: `%TEMP%\dial-control\dial-control.log`; macOS: `$TMPDIR/dial-control/dial-control.log` (clicks, media-session events, focus changes).
- Linux: run the binary from a terminal; the dial and context pipelines log to stdout.
- Android: `adb logcat -s DialControl`, plus the live log on the main screen.

## 7. Troubleshooting

| Problem | Cause / fix |
|---|---|
| Dial clicks also toggle your player (Linux) | The `evdev` grab failed: check udev permissions and reconnect the earbuds. |
| Window shows "localhost not found" | Binary built without the frontend; use the production build above. |
| Events fire twice | Emit each Tauri event once (`app.emit` already broadcasts to all webviews). |
| `GLIBC_2.39 not found` | Build or run on a newer distribution. |

## 8. Creating a release

CI workflows only build and upload *artifacts* (they are manual by design), so a GitHub Release is assembled by hand.

1. **Bump the version** in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json` (keep them equal), and the version strings in `scripts/build-packages.sh` / `dist-packages/PKGBUILD`.
2. **Update `CHANGELOG.md`**: move *Unreleased* entries under the new version and date.
3. **Run every check** from [CONTRIBUTING.md](../CONTRIBUTING.md#checks).
4. **Commit and tag**:
   ```bash
   git commit --author="Yuvraj675 <yuviyg675@gmail.com>" -am "chore(release): v1.1.0"
   git tag -a v1.1.0 -m "Dial Control v1.1.0"
   git push origin main --tags
   ```
5. **Build the installers**:
   - Linux: `npx @tauri-apps/cli build --no-bundle && ./scripts/build-packages.sh` (files land in `dist-packages/`).
   - Windows / macOS: GitHub → **Actions** → *Build Windows Release* / *Build macOS Release* → **Run workflow**, then download the artifacts from the finished run.
6. **Publish**: GitHub → **Releases → Draft a new release**, pick the tag, paste the changelog section, attach the `.deb`, `.rpm`, `.pkg.tar.zst`, `.AppImage`, `.exe`, `.msi` and `.dmg`, then publish. Or from the CLI:
   ```bash
   gh release create v1.1.0 dist-packages/*.{deb,rpm,zst,AppImage} <downloaded .exe/.msi/.dmg> \
     --title "Dial Control v1.1.0" --notes-file release-notes.md
   ```
7. Tick *Set as a pre-release* until the macOS/Windows builds have been tried on real hardware. Android is not part of releases yet.
