# CMF Buds Pro 2 — Smart Dial Control System

Turn your **CMF Buds Pro 2** case dial into a versatile physical controller for your laptop.

- **Tap the Dial**: An on-screen virtual radial dial pops up.
- **Rotate the Dial**: Rotates through options (Volume, Brightness, Media, Desktops, etc.).
- **Click Again**: Confirms and executes the action.

---

## Project Structure

```
.
├── docs/
│   ├── PROTOCOL_SPEC.md       # Complete reverse-engineered RFCOMM wire protocol
│   └── DEVELOPMENT_GUIDE.md   # Step-by-step setup, usage, and extension guide
├── tools/
│   ├── proto.py               # Protocol parser, packet builder & CRC-16 engine
│   ├── probe.py               # RFCOMM channel scanner & handshake prober
│   ├── listen.py              # Live event stream monitor for dial rotations/clicks
│   └── virtual_dial.py        # Complete on-screen radial UI & background daemon
└── extracted/                 # Extracted APK assets, DEX files, and configs
```

---

## Quick Start

### 1. Test the Virtual Dial UI immediately (without Bluetooth)
```bash
python3 tools/virtual_dial.py --test
```
*(Use Space to open, Left/Right arrows to rotate, Enter to select)*

### 2. Connect to Hardware
```bash
# 1. Connect your earbuds in Bluetooth
bluetoothctl connect 2C:BE:EE:31:11:77

# 2. Probe active RFCOMM channel
python3 tools/probe.py

# 3. Stream live dial events
python3 tools/listen.py --channel 15

# 4. Launch the Virtual Dial system
python3 tools/virtual_dial.py --channel 15
```

---

## Documentation

- 📖 **[Protocol Specification](docs/PROTOCOL_SPEC.md)**: Packet structure, command tables (`0xE014`, `0xC0xx`, `0xF0xx`), gesture codes, and CRC-16 algorithm.
- 🛠️ **[Development Guide](docs/DEVELOPMENT_GUIDE.md)**: How to add custom macros, shortcuts, app launchers, and mobile companion architecture.
