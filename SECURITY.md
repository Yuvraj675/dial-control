# Security Policy

## Supported versions

Only the latest release receives security fixes.

## Reporting a vulnerability

Please **do not open a public issue**. Use GitHub's private reporting: **Security → Report a vulnerability** on the [repository](https://github.com/Yuvraj675/dial-control/security/advisories/new). Include affected OS, version, and reproduction steps. You can expect an acknowledgement within a week.

## Scope notes

Dial Control runs entirely locally and makes no network requests. It does need powerful local permissions to do its job:

- **Linux**: `/dev/uinput` and the earbuds' input node (via a `udev` rule), session/system D-Bus.
- **Windows**: low-level keyboard hook, Raw Input, audio endpoint access; no administrator rights.
- **macOS**: Accessibility permission for the event tap.
- **Android**: Accessibility service, notification access, optional system-settings and DND access.
- Slices of type *command*, *launch* and *macro* run what the user configured, with the user's privileges. Treat imported or shared `profiles.json` files like scripts.
