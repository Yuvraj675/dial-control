#!/usr/bin/env bash
# Dial Control installer for Linux and macOS (Windows: see install.ps1, or run this in Git Bash).
#
#   curl -fsSL https://github.com/Yuvraj675/dial-control/releases/latest/download/install.sh | bash
#
# Options (pass after `bash -s --`):
#   --version X.Y.Z   install a specific release instead of the latest
#   --appimage        Linux: install the portable AppImage into ~/.local instead of a distro package
#   --dry-run         download and verify, but print the install commands instead of running them
#   -h, --help        show this help
#
# Environment: DIAL_CONTROL_VERSION, DIAL_CONTROL_BASE_URL (testing).
#
# What it does: finds the latest release tag, picks the right package for your OS and distribution,
# downloads it together with SHA256SUMS, verifies the checksum, then installs it. Read it first if
# you like: curl -fsSL <url> -o install.sh && less install.sh && bash install.sh
set -euo pipefail

REPO="Yuvraj675/dial-control"
BASE="${DIAL_CONTROL_BASE_URL:-https://github.com/$REPO/releases}"
VERSION="${DIAL_CONTROL_VERSION:-}"
FORCE_APPIMAGE=0
DRY_RUN=0

if [ -t 1 ]; then B=$'\033[1m'; G=$'\033[32m'; Y=$'\033[33m'; R=$'\033[31m'; N=$'\033[0m'; else B=; G=; Y=; R=; N=; fi
info() { printf '%s==>%s %s\n' "$B" "$N" "$*"; }
ok()   { printf '%s ok%s %s\n' "$G" "$N" "$*"; }
warn() { printf '%s warn%s %s\n' "$Y" "$N" "$*" >&2; }
die()  { printf '%s error%s %s\n' "$R" "$N" "$*" >&2; exit 1; }

usage() {
  cat <<'EOF2'
Dial Control installer (Linux, macOS; Windows via install.ps1)

  curl -fsSL https://github.com/Yuvraj675/dial-control/releases/latest/download/install.sh | bash

Options (after `bash -s --`):
  --version X.Y.Z   install a specific release instead of the latest
  --appimage        Linux: install the portable AppImage into ~/.local
  --dry-run         download and verify, but only print the install commands
  -h, --help        show this help
EOF2
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || die "--version needs a value"; VERSION="${2#v}"; shift 2 ;;
    --version=*) VERSION="${1#--version=}"; VERSION="${VERSION#v}"; shift ;;
    --appimage) FORCE_APPIMAGE=1; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) usage; exit 0 ;;
    *) die "unknown option: $1 (try --help)" ;;
  esac
done

have() { command -v "$1" >/dev/null 2>&1; }

fetch() { # fetch URL DEST
  if have curl; then curl -fL --retry 3 --proto '=https,http' -sS -o "$2" "$1"
  elif have wget; then wget -q -O "$2" "$1"
  else die "curl or wget is required"; fi
}

SUDO=""
as_root() {
  if [ "$DRY_RUN" = 1 ]; then printf '   [dry-run] %s\n' "$*"; return 0; fi
  if [ "$(id -u)" -eq 0 ]; then "$@"
  elif have sudo; then sudo "$@"
  else die "root privileges are needed for: $* (install sudo or run as root)"; fi
}

run() { if [ "$DRY_RUN" = 1 ]; then printf '   [dry-run] %s\n' "$*"; else "$@"; fi; }

latest_version() {
  have curl || die "curl is required to look up the latest release"
  local url
  url="$(curl -fsSLI -o /dev/null -w '%{url_effective}' "$BASE/latest")" || die "could not reach GitHub"
  case "$url" in
    */tag/v*) printf '%s' "${url##*/tag/v}" ;;
    *) die "no published release found at $BASE (has a release been published yet?)" ;;
  esac
}

sha256_of() {
  if have sha256sum; then sha256sum "$1" | cut -d' ' -f1
  elif have shasum; then shasum -a 256 "$1" | cut -d' ' -f1
  else die "sha256sum or shasum is required to verify the download"; fi
}

TMP=""
cleanup() { if [ -n "$TMP" ] && [ -d "$TMP" ]; then rm -rf "$TMP"; fi; }
trap cleanup EXIT

download_verified() { # download_verified ASSET -> path in $TMP
  local asset="$1" url="$BASE/download/v$VERSION"
  info "Downloading $asset"
  fetch "$url/$asset" "$TMP/$asset" || die "download failed: $url/$asset"
  [ -s "$TMP/SHA256SUMS" ] || fetch "$url/SHA256SUMS" "$TMP/SHA256SUMS" || die "could not download SHA256SUMS"
  local want got
  want="$(awk -v f="$asset" '$2==f || $2=="*"f {print $1}' "$TMP/SHA256SUMS" | head -1)"
  [ -n "$want" ] || die "$asset is not listed in SHA256SUMS"
  got="$(sha256_of "$TMP/$asset")"
  [ "$want" = "$got" ] || die "checksum mismatch for $asset (expected $want, got $got); not installing"
  ok "checksum verified"
}

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  MINGW*|MSYS*|CYGWIN*)
    info "Windows shell detected; running the PowerShell installer"
    have powershell.exe || die "powershell.exe not found"
    exec powershell.exe -NoProfile -ExecutionPolicy Bypass -Command \
      "\$env:DIAL_CONTROL_VERSION='$VERSION'; irm '$BASE/latest/download/install.ps1' | iex"
    ;;
esac

TMP="$(mktemp -d 2>/dev/null || mktemp -d -t dial-control)"

[ -n "$VERSION" ] || { info "Looking up the latest release"; VERSION="$(latest_version)"; }
info "Dial Control v$VERSION on $OS/$ARCH"

install_linux() {
  case "$ARCH" in x86_64|amd64) ;; *) die "only x86_64 Linux builds are published (this machine is $ARCH)";; esac

  local glibc
  glibc="$(ldd --version 2>/dev/null | head -1 | grep -oE '[0-9]+\.[0-9]+$' || true)"
  if [ -n "$glibc" ]; then
    if [ "$(printf '%s\n2.39\n' "$glibc" | sort -V | head -1)" != "2.39" ]; then
      warn "glibc $glibc found, but Dial Control needs 2.39 or newer (Ubuntu 24.04+, Fedora 40+, current Arch)"
    fi
  fi

  local id="" like="" family=""
  if [ -r /etc/os-release ]; then
    id="$(. /etc/os-release; printf '%s' "${ID:-}")"
    like="$(. /etc/os-release; printf '%s' "${ID_LIKE:-}")"
  fi
  local ids=" $id $like "
  case "$ids" in
    *" debian "*|*" ubuntu "*) family=deb ;;
    *" arch "*) family=arch ;;
    *" suse "*|*" opensuse "*) family=suse ;;
    *" fedora "*|*" rhel "*|*" centos "*) family=rpm ;;
  esac
  [ "$FORCE_APPIMAGE" = 1 ] && family=appimage
  [ -n "$family" ] || family=appimage
  info "Distribution: ${id:-unknown} -> $family package"

  local asset
  case "$family" in
    deb)
      asset="dial-control_${VERSION}_amd64.deb"
      download_verified "$asset"
      have apt-get || die "apt-get not found"
      as_root apt-get install -y "$TMP/$asset" ;;
    rpm)
      asset="dial-control-${VERSION}-1.x86_64.rpm"
      download_verified "$asset"
      if have dnf; then as_root dnf install -y "$TMP/$asset"
      elif have yum; then as_root yum install -y "$TMP/$asset"
      else as_root rpm -Uvh "$TMP/$asset"; fi ;;
    suse)
      asset="dial-control-${VERSION}-1.x86_64.rpm"
      download_verified "$asset"
      as_root zypper --non-interactive --no-gpg-checks install "$TMP/$asset" ;;
    arch)
      asset="dial-control-${VERSION}-1-x86_64.pkg.tar.zst"
      download_verified "$asset"
      as_root pacman -U --noconfirm "$TMP/$asset" ;;
    appimage)
      asset="Dial-Control-${VERSION}-amd64.AppImage"
      download_verified "$asset"
      install_appimage "$TMP/$asset" ;;
  esac

  echo
  ok "Dial Control v$VERSION installed."
  cat <<'EOF'

Next steps
  * In your earbuds' companion app set: double press = Next track, triple press = Previous track.
  * GNOME on Wayland: log out and back in once so the bundled extension loads.
  * If your earbuds were already connected, disconnect and reconnect them once.
  * Launch "Dial Control" from your app menu, or run: dial-control
  * No earbuds? Press Super+Shift+D to open the wheel from the keyboard.
EOF
}

install_appimage() { # install_appimage FILE
  local bindir="${HOME}/.local/bin" appdir="${HOME}/.local/share/applications"
  run mkdir -p "$bindir" "$appdir"
  run install -m 755 "$1" "$bindir/dial-control"
  if [ "$DRY_RUN" != 1 ]; then
    cat > "$appdir/dial-control.desktop" <<EOF
[Desktop Entry]
Name=Dial Control
Comment=Radial dial controller for earbuds
Exec=$bindir/dial-control
Icon=dial-control
Terminal=false
Type=Application
Categories=Utility;AudioVideo;Settings;
StartupWMClass=dial-control
EOF
  fi
  # udev rule: grant the logged-in user access to the dial's input node and /dev/uinput.
  local rule='KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0660", TAG+="uaccess", OPTIONS+="static_node=uinput"
SUBSYSTEM=="input", KERNEL=="event*", ATTRS{name}=="*CMF Buds Pro 2*", MODE="0660", TAG+="uaccess"'
  if [ "$DRY_RUN" = 1 ]; then
    printf '   [dry-run] write /etc/udev/rules.d/99-dial-control.rules and reload udev\n'
  elif [ "$(id -u)" -eq 0 ] || have sudo; then
    printf '%s\n' "$rule" | as_root tee /etc/udev/rules.d/99-dial-control.rules >/dev/null
    as_root udevadm control --reload-rules || true
    as_root udevadm trigger --subsystem-match=input --subsystem-match=misc || true
  else
    warn "no sudo: add the udev rule manually (see README), or add your user to the 'input' group"
  fi
  case ":$PATH:" in *":$bindir:"*) ;; *) warn "$bindir is not on your PATH; add it to run 'dial-control' from a terminal" ;; esac
}

install_macos() {
  case "$ARCH" in arm64) ;; *) die "the published macOS build is Apple Silicon (arm64) only; this Mac is $ARCH" ;; esac
  local asset="Dial-Control_${VERSION}_aarch64.dmg"
  download_verified "$asset"
  local mnt="$TMP/mnt"
  info "Installing to /Applications"
  if [ "$DRY_RUN" = 1 ]; then
    printf '   [dry-run] hdiutil attach, copy "Dial Control.app" to /Applications, detach, remove quarantine flag\n'
  else
    mkdir -p "$mnt"
    hdiutil attach -nobrowse -quiet -readonly -mountpoint "$mnt" "$TMP/$asset" || die "could not mount the disk image"
    local app
    app="$(find "$mnt" -maxdepth 1 -name '*.app' | head -1)"
    [ -n "$app" ] || { hdiutil detach -quiet "$mnt" || true; die "no .app found in the disk image"; }
    pkill -x dial-control 2>/dev/null || true
    rm -rf "/Applications/$(basename "$app")" 2>/dev/null || sudo rm -rf "/Applications/$(basename "$app")"
    cp -R "$app" /Applications/ 2>/dev/null || sudo cp -R "$app" /Applications/
    hdiutil detach -quiet "$mnt" || true
    xattr -dr com.apple.quarantine "/Applications/$(basename "$app")" 2>/dev/null || sudo xattr -dr com.apple.quarantine "/Applications/$(basename "$app")" || true
  fi
  echo
  ok "Dial Control v$VERSION installed."
  cat <<'EOF'

Next steps
  * Open "Dial Control" (menu-bar app) and grant Accessibility access when asked:
    System Settings > Privacy & Security > Accessibility.
  * In your earbuds' companion app set: double press = Next track, triple press = Previous track.
  * No earbuds? Press Cmd+Shift+D to open the wheel from the keyboard.
EOF
}

case "$OS" in
  Linux)  install_linux ;;
  Darwin) install_macos ;;
  *) die "unsupported OS: $OS (Windows users: irm $BASE/latest/download/install.ps1 | iex)" ;;
esac
