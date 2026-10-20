#!/usr/bin/env bash
# Setup udev rules for CMF Buds Pro 2 Smart Dial
# Grants the logged-in desktop user access to AVRCP dial events without requiring sudo!

set -e

RULE_FILE="/etc/udev/rules.d/99-cmf-dial.rules"

echo "[*] Installing CMF Buds Pro 2 udev rule to $RULE_FILE..."

cat << 'EOF' | sudo tee "$RULE_FILE" > /dev/null
# CMF Buds Pro 2 Smart Dial - Grant desktop user access without root
KERNEL=="event*", ATTRS{name}=="CMF Buds Pro 2 (AVRCP)", TAG+="uaccess", MODE="0660"
SUBSYSTEM=="input", ATTRS{name}=="CMF Buds Pro 2 (AVRCP)", TAG+="uaccess", MODE="0660"
EOF

echo "[*] Reloading udev rules..."
sudo udevadm control --reload-rules
sudo udevadm trigger

# Also add user to input group as permanent fallback
sudo usermod -aG input "$USER" || true

echo "[+] Successfully configured! You can now launch Dial Control directly from your Applications menu without sudo."
