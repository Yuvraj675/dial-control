# Contributing to Dial Control

Thanks for helping! Dial Control is a cross-platform product (Linux, Windows, macOS, Android), so the rules below keep one OS from breaking another.

## Ground rules

1. **Cross-platform first.** Add behaviour as a trait in `src-tauri/src/platform/traits.rs`, implement it for every OS, and only then add desktop-specific workarounds.
2. **Never regress another platform.** Run every check below before opening a PR.
3. **Put decisions in pure functions** (`platform/dial_logic.rs`, `keymap.rs`, `common.rs`) and add a unit test, so Windows and macOS logic is testable on any host.
4. **Research before probing.** Check official OS docs and how mature crates solve a problem before experimenting on a machine.
5. **Keep the repo clean.** No binaries, APKs, dumps or `temp/` content. Emit each Tauri event once.

Read [CLAUDE.md](CLAUDE.md) (architecture and archived dead ends) and [docs/DEVELOPMENT_GUIDE.md](docs/DEVELOPMENT_GUIDE.md) first.

## Checks

```bash
npm run build && npm test
cargo test  --manifest-path src-tauri/Cargo.toml --lib
cargo check --manifest-path src-tauri/Cargo.toml
cargo check --target x86_64-pc-windows-msvc --manifest-path src-tauri/Cargo.toml
CC_aarch64_apple_darwin=clang cargo check --target aarch64-apple-darwin --manifest-path src-tauri/Cargo.toml
cd android && ./gradlew testDebugUnitTest assembleDebug   # if android/ changed
cd website && npm ci && npm run build                      # if website/ or src/components/hud/ changed
```

## Commits and pull requests

- Use [Conventional Commits](https://www.conventionalcommits.org/): `feat(scope): ...`, `fix(scope): ...`, `docs: ...`.
- One logical change per PR; describe what you tested and on which OS.
- Bug reports from hardware help most: include OS, earbuds firmware, and the diagnostics log (see the README's Troubleshooting).

By contributing you agree that your work is licensed under the [MIT License](LICENSE).
