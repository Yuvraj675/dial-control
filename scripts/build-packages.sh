#!/usr/bin/env bash
set -e

DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$DIR"

echo "==> [1/4] Compiling release frontend and binary..."
npx @tauri-apps/cli build --no-bundle

mkdir -p dist-packages temp/pkgconfig

# Setup pkgconfig for ayatana-appindicator
cat << 'EOF' > temp/pkgconfig/ayatana-appindicator3-0.1.pc
prefix=/usr
exec_prefix=${prefix}
libdir=/usr/lib64
includedir=${prefix}/include

Name: ayatana-appindicator3-0.1
Description: Ayatana Application Indicators library
Version: 0.5.94
Requires: gtk+-3.0
Libs: -L${libdir} -layatana-appindicator3
Cflags: -I${includedir}/libayatana-appindicator3-0.1
EOF
cp temp/pkgconfig/ayatana-appindicator3-0.1.pc temp/pkgconfig/appindicator3-0.1.pc
sed -i 's/ayatana-//g' temp/pkgconfig/appindicator3-0.1.pc
export PKG_CONFIG_PATH="$DIR/temp/pkgconfig:$PKG_CONFIG_PATH"

echo "==> [2/4] Generating RPM and AppImage packages via Tauri..."
npx @tauri-apps/cli build --bundles rpm,appimage || true

if [ -f "src-tauri/target/release/bundle/rpm/Dial Control-1.0.0-1.x86_64.rpm" ]; then
    cp "src-tauri/target/release/bundle/rpm/Dial Control-1.0.0-1.x86_64.rpm" dist-packages/dial-control-1.0.0-1.x86_64.rpm
fi
if [ -f "src-tauri/target/release/bundle/appimage/Dial Control_1.0.0_amd64.AppImage" ]; then
    cp "src-tauri/target/release/bundle/appimage/Dial Control_1.0.0_amd64.AppImage" dist-packages/Dial-Control-1.0.0-amd64.AppImage
    chmod +x dist-packages/Dial-Control-1.0.0-amd64.AppImage
fi

echo "==> [3/4] Assembling Debian (.deb) package..."
mkdir -p temp/deb-build/DEBIAN
mkdir -p temp/deb-build/usr/bin
mkdir -p temp/deb-build/usr/share/applications
mkdir -p temp/deb-build/usr/share/icons/hicolor/512x512/apps
mkdir -p temp/deb-build/usr/lib/udev/rules.d

cp src-tauri/target/release/dial-control temp/deb-build/usr/bin/dial-control
chmod +x temp/deb-build/usr/bin/dial-control

cat << 'EOF' > temp/deb-build/usr/share/applications/dial-control.desktop
[Desktop Entry]
Name=Dial Control
GenericName=Radial Dial Controller
Comment=Context-Aware Radial Dial Controller for CMF Buds Pro 2
Exec=dial-control
Icon=dial-control
Terminal=false
Type=Application
Categories=Utility;AudioVideo;Settings;
StartupWMClass=dial-control
Keywords=dial;cmf;buds;radial;controller;audio;volume;
EOF

cp src-tauri/icons/icon.png temp/deb-build/usr/share/icons/hicolor/512x512/apps/dial-control.png

cat << 'EOF' > temp/deb-build/usr/lib/udev/rules.d/99-dial-control.rules
# Grant active desktop user access to /dev/uinput for shortcut emulation
KERNEL=="uinput", SUBSYSTEM=="misc", TAG+="uaccess", OPTIONS+="static_node=uinput"

# Grant active desktop user access to CMF Buds Pro 2 AVRCP hardware input
SUBSYSTEM=="input", ATTRS{name}=="*CMF Buds Pro 2*", TAG+="uaccess", MODE="0660"
SUBSYSTEM=="input", ATTRS{name}=="*AVRCP*", TAG+="uaccess", MODE="0660"
EOF

cat << 'EOF' > temp/deb-build/DEBIAN/control
Package: dial-control
Version: 1.0.0
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Yuvraj Gupta <yuviyg675@gmail.com>
Installed-Size: 18000
Depends: libwebkit2gtk-4.1-0 | libwebkit2gtk-4.0-37, libgtk-3-0, libayatana-appindicator3-1 | libappindicator3-1
Description: Context-Aware Radial Dial Controller for CMF Buds Pro 2
 Dial Control provides an intuitive, Nothing OS-styled radial HUD and
 context-aware profiles for desktop controls, audio, media, and applications.
EOF

cat << 'EOF' > temp/deb-build/DEBIAN/postinst
#!/bin/sh
set -e
if command -v udevadm >/dev/null 2>&1; then
    udevadm control --reload-rules || true
    udevadm trigger || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q /usr/share/icons/hicolor || true
fi
exit 0
EOF
chmod 755 temp/deb-build/DEBIAN/postinst

cd temp/deb-build
echo "2.0" > debian-binary
tar --numeric-owner --owner=0 --group=0 -czvf control.tar.gz -C DEBIAN control postinst
tar --numeric-owner --owner=0 --group=0 -czvf data.tar.gz usr
ar rcs "$DIR/dist-packages/dial-control_1.0.0_amd64.deb" debian-binary control.tar.gz data.tar.gz
cd "$DIR"

echo "==> [4/4] Assembling Arch Linux package..."
mkdir -p temp/arch-build/usr
cp -r temp/deb-build/usr/* temp/arch-build/usr/

cat << 'EOF' > temp/arch-build/.PKGINFO
pkgname = dial-control
pkgver = 1.0.0-1
pkgdesc = Context-Aware Radial Dial Controller for CMF Buds Pro 2
url = https://github.com/Yuvraj675/dial-control
builddate = 1792600000
packager = Yuvraj Gupta <yuviyg675@gmail.com>
size = 18500000
arch = x86_64
license = MIT
depend = webkit2gtk-4.1
depend = gtk3
depend = libayatana-appindicator
EOF

cd temp/arch-build
tar --numeric-owner --owner=0 --group=0 -cf - .PKGINFO usr | zstd -19 -T0 -o "$DIR/dist-packages/dial-control-1.0.0-1-x86_64.pkg.tar.zst"
cd "$DIR"

rm -rf temp/deb-build temp/arch-build temp/pkgconfig

echo "==> All packages ready in dist-packages/:"
ls -lh dist-packages/
