# CMF Buds Pro 2 Smart Dial — Development Guide

This guide walks you through developing, running, and extending the **Laptop & Mobile Smart Dial Control System** for the CMF Buds Pro 2.

---

## 1. Quick Start

### 1.1 Prerequisites
On Linux, Python 3.9+ includes native Bluetooth support via `socket.AF_BLUETOOTH` and `socket.BTPROTO_RFCOMM`.

```bash
cd /home/yuvraj/Documents/Code_Projects/dial-control/tools
chmod +x probe.py listen.py virtual_dial.py
```

### 1.2 Test the Virtual Dial UI Immediately
You can test the floating radial menu without connecting Bluetooth:

```bash
python3 virtual_dial.py --test
```
- **Space** / **Tab**: Open / Toggle the Virtual Dial
- **Right** / **Up**: Rotate selection Clockwise
- **Left** / **Down**: Rotate selection Counter-Clockwise
- **Enter**: Confirm selected option & execute action
- **Esc**: Dismiss the dial

---

## 2. Connecting to Hardware

### Step 1: Check Bluetooth Pairing
Ensure your CMF Buds Pro 2 is paired to your laptop:
```bash
bluetoothctl devices | grep -i "CMF Buds Pro 2"
# Found: 2C:BE:EE:31:11:77
```

### Step 2: Ensure Connection is Free
> [!IMPORTANT]
> The Nothing RFCOMM socket can only be connected to **one host application at a time**.
> Disconnect the earbuds from your mobile phone or close the Nothing X app before scanning from your laptop.

Connect to your earbuds via BlueZ:
```bash
bluetoothctl connect 2C:BE:EE:31:11:77
```

### Step 3: Discover the Active RFCOMM Channel
Run the channel scanner:
```bash
python3 probe.py --mac 2C:BE:EE:31:11:77
```
It tests common channels (15, 28, 17, 1-30) and prints the active Nothing protocol channel.

### Step 4: Stream Dial Events in Real Time
Once the channel is confirmed (e.g. `15` or `28`):
```bash
python3 listen.py --mac 2C:BE:EE:31:11:77 --channel <CHANNEL>
```
Now spin the dial or click the button on your charging case — you will see live decoded events printed in green:
```
[DIAL EVENT] Payload: [0a 01] -> DIAL ROTATE: CLOCKWISE (+) (step: 1)
[DIAL EVENT] Payload: [01 01] -> DIAL SINGLE CLICK
```

### Step 5: Launch the Virtual Dial System
```bash
python3 virtual_dial.py --mac 2C:BE:EE:31:11:77 --channel <CHANNEL>
```
Now:
1. Tap the Smart Dial on the case -> **Virtual radial dial pops up on screen**.
2. Turn the Smart Dial -> **Highlights change smoothly around the circle**.
3. Tap or double-tap again -> **Action triggers (Volume, Media, Brightness, etc.) and dial closes**.

---

## 3. Extending Actions

In `tools/virtual_dial.py`, actions are defined in `DIAL_ACTIONS`:

```python
DIAL_ACTIONS = [
    ("🔊 Volume", "volume"),
    ("☀️ Brightness", "brightness"),
    ("⏯️ Media Play/Pause", "media_play"),
    ("⏭️ Next Track", "media_next"),
    ("⏮️ Prev Track", "media_prev"),
    ("📜 Scroll Mode", "scroll"),
    ("🖥️ Switch Workspace", "workspace"),
    ("🔇 Mute Audio", "mute"),
]
```

Add your custom handler inside `execute_action(self, action_key)`:

```python
def execute_action(self, action_key: str):
    if action_key == "my_custom_script":
        subprocess.run(["/path/to/my_script.sh"])
    elif action_key == "app_launcher":
        subprocess.run(["gtk-launch", "firefox.desktop"])
    elif action_key == "workspace":
        # Example for GNOME / Wayland / i3 / Hyprland
        subprocess.run(["hyprctl", "dispatch", "workspace", "+1"])
```

---

## 4. Reverse Engineering Findings Summary

From decompiling `com.nothing.smartcenter_3.10.0` with REA and manual DEX analysis:

| Component | Code / Value | Description |
|---|---|---|
| **Device Model** | `B172` | Internal codename/model for CMF Buds Pro 2 |
| **ODM / SoC** | BES (Bestechnic) | Embedded Bluetooth microcontroller in buds |
| **Service UUID** | `aeac4a03-dff5-498f-843a-34487cf133eb` | Nothing proprietary SPP service |
| **Dial Device Target** | `deviceType = 4` | Case Dial (`ControlConfigurationEntity`) |
| **Dial Center Button** | `button = 1` | Function button in dial center |
| **Dial Gestures** | `1` (click), `2` (double), `7` (hold), `10` (rotate) | Physical actions on the case dial |
| **Dial Event ID** | `0xE014` (`EVENT_MAGIC_BUTTON`) | Asynchronous RFCOMM notification for dial input |
| **Rotate Operation** | `23` (`OPERATION_ROTATE_CONTROL`) | Action bound to rotation |
| **Frame Header** | `0x55` | Start-of-frame delimiter |
| **Checksum** | CRC-16 / MODBUS | Polynomial `0xA001`, initial `0xFFFF` |

---

## 5. Troubleshooting & FAQ

### Q: Socket connection fails with "Device or resource busy"
**Cause**: Another process (like a previous instance of `listen.py` or a connected phone) is holding the RFCOMM channel open.
**Fix**:
1. Check running python processes: `pgrep -af python3`
2. Disconnect and reconnect in bluetoothctl:
   ```bash
   bluetoothctl disconnect 2C:BE:EE:31:11:77
   bluetoothctl connect 2C:BE:EE:31:11:77
   ```

### Q: No dial events arrive while turning the dial
**Cause**: The earbuds might not have activated the dial session yet, or the dial is locked.
**Fix**:
1. Double-press and hold the dial on the case for 2 seconds to unlock (Case Lock feature).
2. Ensure `SET_PROTOCOL_ACTIVATED` (`0xF001`) was sent during connection handshake (handled automatically by `proto.py`).
