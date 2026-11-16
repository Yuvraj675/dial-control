# Dial Control

I want to understand an app called Dial Control. Read the notes below, then explain it to me in plain language, step by step, and answer my questions. Use only these notes. If something is not covered, say you do not know and point me to the GitHub link.

> Dial Control turns the rotary dial on the CMF Buds Pro 2 earbud case, or a keyboard shortcut, into a radial menu that launches apps, runs shortcuts and macros, scrolls, and sets volume and brightness. It is free and open source (MIT).

## Links

- Website: {{SITE}}
- Source code, documentation and issues: {{REPO}}
- Downloads: {{REPO}}/releases/latest

## What it does

- With the wheel closed, the earbuds behave as usual: turning the dial changes volume, a click plays or pauses, and a triple press goes back a track.
- A double press, or a keyboard shortcut, opens a radial wheel on screen. Turning the dial moves the highlight, a click runs the highlighted option, and a double press closes the wheel again.
- The wheel changes with the app in front. VS Code gets a Code wheel, a file manager gets Files, a browser gets Browser, and so on. A Home option switches dials by hand.
- Each option (a slice) can press a hotkey, open an app, file or link, run a shell command, type a text snippet, or run a macro: several steps in order, with waits between them.
- Built-in modes: scroll (the dial becomes a mouse wheel), volume (a live gauge that restores your own level afterwards) and brightness (steps the display, on laptop panels and external monitors).
- Dials and slices are edited in a Settings window. A tray icon keeps the app running in the background.

## No earbuds needed

Press Super+Shift+D (Win+Shift+D on Windows, Cmd+Shift+D on macOS) to open the wheel from the keyboard. Arrow keys move the highlight, Enter runs it, Esc closes it. The shortcut can be changed in Settings.

## How it works

The CMF Buds Pro 2 dial sends standard Bluetooth AVRCP volume and media commands, not a special protocol. Dial Control intercepts them on Windows, macOS and Linux and turns them into the wheel. It is built with Tauri v2, Rust and React, and the app makes no network requests.

## Install

- Linux and macOS: `curl -fsSL {{REPO}}/releases/latest/download/install.sh | bash`
- Windows (PowerShell): `irm {{REPO}}/releases/latest/download/install.ps1 | iex`
- The script finds your system, verifies a SHA-256 checksum and installs the latest release.
- Packages: Windows 10 and 11 (64-bit) as .exe or .msi; macOS 14 or newer on Apple Silicon as .dmg; Linux as .deb, .rpm, .pkg.tar.zst or .AppImage on any distribution with glibc 2.39 or newer.
- Optional earbuds setup: in the earbuds' own app, set double press to Next track and triple press to Previous track.

## Good to know

- The installers are not code-signed yet, so Windows SmartScreen and macOS Gatekeeper show a warning. On Windows choose More info, then Run anyway. The macOS install script removes the quarantine flag.
- An Android app is a work in progress and has no release yet.
- No account is needed. The license is MIT.
- It is an independent project, not affiliated with Nothing Technology Limited or CMF.

## Help me with

Ask what I want to do, then explain whether Dial Control fits, how to set it up on my system, and how to make my own slices. Offer a short example of each if it helps.
