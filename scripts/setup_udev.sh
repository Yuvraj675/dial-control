#!/usr/bin/env bash
# Setup udev rules for CMF Buds Pro 2 Smart Dial
# Grants the logged-in desktop user access to AVRCP dial events without requiring sudo!

set -e

RULE_FILE="/etc/udev/rules.d/99-cmf-dial.rules"

echo "[*] Installing CMF Buds Pro 2 udev rule to $RULE_FILE..."

cat << 'EOF' | sudo tee "$RULE_FILE" > /dev/null
# CMF Buds Pro 2 Smart Dial - Grant desktop user access without root
KERNEL=="event*", ATTRS{name}=="*CMF Buds Pro 2*", MODE="0666", TAG+="uaccess"
SUBSYSTEM=="input", ATTRS{name}=="*CMF Buds Pro 2*", MODE="0666", TAG+="uaccess"
KERNEL=="uinput", SUBSYSTEM=="misc", MODE="0666", TAG+="uaccess"
EOF

echo "[*] Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger --subsystem-match=input

# Immediately grant ACL on existing CMF event device if connected
for dev in /dev/input/event*; do
    if udevadm info "$dev" 2>/dev/null | grep -iq "CMF Buds Pro 2"; then
        echo "[+] Setting permissions on active device $dev..."
        sudo chmod 0666 "$dev" || true
        sudo setfacl -m u:"$USER":rw "$dev" 2>/dev/null || true
    fi
done

# Ensure /dev/uinput permissions
if [ -e /dev/uinput ]; then
    sudo chmod 0666 /dev/uinput || true
    sudo setfacl -m u:"$USER":rw /dev/uinput 2>/dev/null || true
fi

echo "[+] Successfully configured! You can now launch Dial Control directly as your standard user without sudo."
