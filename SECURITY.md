# Security Policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private reporting: **Security → Report a vulnerability** on the [repository](https://github.com/Yuvraj675/dial-control/security/advisories/new). Include affected OS, version, and reproduction steps. You can expect an acknowledgement within a week.

## Scope notes

Dial Control runs entirely locally: the UI is bundled (no remote fonts or scripts, enforced by a strict Content-Security-Policy) and the app makes no network requests. It does need powerful local permissions to do its job:

- **Linux**: `/dev/uinput` and the earbuds' input node (via a `udev` rule), session/system D-Bus.
- **Windows**: low-level keyboard hook, Raw Input, audio endpoint access; no administrator rights.
- **macOS**: Accessibility permission for the event tap.
- **Android**: Accessibility service, notification access, optional system-settings and DND access.
- Slices of type *command*, *launch* and *macro* run what the user configured, with the user's privileges. Treat imported or shared `profiles.json` files like scripts.

### Hardening in place

- udev rules grant access with `uaccess` and mode `0660` (never world-writable), so only the logged-in desktop user can use `/dev/uinput` and the dial's input node.
- The single-instance / `--toggle` socket lives in `$XDG_RUNTIME_DIR` (per-user, `0700`) with mode `0600`; the Windows equivalent is loopback-only and accepts only the `show` and `toggle` commands.
- The Tauri capability set is minimal (no shell, filesystem or HTTP plugins are exposed to the webview).
- The install scripts download only over HTTPS and verify a SHA-256 checksum before installing; release workflows run with read-only tokens except the job that publishes the release.
- Dependencies are scanned with `pnpm audit` and `cargo audit`, and Dependabot is enabled.

### Install scripts

`install.sh` and `install.ps1` are attached to every release. The checksum protects against corrupted or tampered downloads, but it comes from the same release, so it does not defend against a compromised release. Review the script first if that matters to you (see the README).
