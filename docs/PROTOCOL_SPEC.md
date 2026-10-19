# CMF Buds Pro 2 (B172) Bluetooth RFCOMM Protocol Specification

> **Source**: Reverse-engineered from Nothing X Android APK (`com.nothing.smartcenter_3.10.0`), decompiled DEX bytecode analysis, and verified community implementations (`SoftARV/cmfctl`, `SoaOaoS/something-x`, `maniacx/BudsLink`).

---

## 1. Hardware & System Topology

```
┌─────────────────────────────────┐
│     CMF Buds Pro 2 Case         │
│  - Rotating Smart Dial (Knob)   │
│  - Center Function Button       │
└───────────────┬─────────────────┘
                │ Pogo pins / internal UART
                ▼
┌─────────────────────────────────┐
│         Active Earbud           │
│  - Bestechnic (BES) BT SoC      │
│  - Model: B172                  │
└───────────────┬─────────────────┘
                │ Bluetooth Classic (BR/EDR)
                │ RFCOMM Socket (SPP Profile)
                ▼
┌─────────────────────────────────┐
│    Host (Laptop / PC / Linux)   │
│  - BlueZ Bluetooth Stack        │
│  - Python socket(AF_BLUETOOTH)  │
└─────────────────────────────────┘
```

### Device Characteristics
- **Product Name**: CMF Buds Pro 2
- **Model ID**: `B172` (SKUs `76`, `77`, `78`, `79`, `80`, `81`, `82`, `83`)
- **FastPair ID**: `F29566`
- **SoC Vendor**: BES (Bestechnic)
- **Supported MAC Prefixes**: `2C:BE:EE`, `2C:BE:EB`, `3C:B0:ED`, `04:5A:3D`
- **Service UUID**: `aeac4a03-dff5-498f-843a-34487cf133eb` (Nothing Proprietary SPP)
- **Standard Serial Port Profile**: `00001101-0000-1000-8000-00805f9b34fb`

---

## 2. Wire Framing Format

Every packet exchanged between host and earbuds adheres to the **0x55 framing protocol**:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|   SOF (0x55)  |  Ctrl (Flags) |     Target    |  Command ID   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
| Command (cont)| Payload Length (u16 LE)       |   FSN (Seq)   |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                   Payload Data (0 to N bytes)                 |
|                              ...                              |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|      CRC-16 / MODBUS (u16 LE) [Present if Ctrl & 0x20]        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

### Field Definitions

| Field | Offset | Length | Type | Description |
|---|---|---|---|---|
| `SOF` | 0 | 1 byte | uint8 | Start of Frame delimiter. Always `0x55`. |
| `Ctrl` | 1 | 1 byte | uint8 | Flags byte: `0x60` indicates CRC-16 is appended. `0x00` indicates no CRC (some async event notifications). Outgoing host frames should always use `0x60`. |
| `Target` | 2 | 1 byte | uint8 | Stream/Device target identifier (`0x01` default, sometimes `0x03` or `0x06`). |
| `Command ID` | 3 | 2 bytes | uint16 LE | Command identifier in **Little-Endian** byte order. |
| `Length` | 5 | 2 bytes | uint16 LE | Payload length in **Little-Endian** byte order. |
| `FSN` | 7 | 1 byte | uint8 | Frame Sequence Number (increments 0x00-0xFF). |
| `Payload` | 8 | N bytes | bytes | Command-specific parameter or response payload. |
| `CRC-16` | 8+N | 2 bytes | uint16 LE | Appended only when `Ctrl & 0x20 != 0` (or `Ctrl == 0x60`). |

### Command ID Convention
- **Host requests** have the high bit set (`0xC000` for queries, `0xF000` for set commands).
- **Device responses** clear the high bit: `Response_ID = Request_ID & 0x7FFF`.
  - For example: Request `0xC001` (`GET_PROTOCOL_VERSION`) receives response `0x4001`.
- **Asynchronous Notifications** emitted by the device always start with `0xE000` (e.g., `0xE001`, `0xE014`).

---

## 3. Checksum Algorithm (CRC-16 / MODBUS)

The checksum is **CRC-16/MODBUS** (Reflected polynomial `0xA001`, Initial value `0xFFFF`), computed over the entire frame from byte 0 (`SOF`) to byte `7 + Payload_Length`.

### Reference Implementation (Python)

```python
def crc16_modbus(data: bytes) -> int:
    crc = 0xFFFF
    for byte in data:
        crc ^= byte
        for _ in range(8):
            if crc & 1:
                crc = (crc >> 1) ^ 0xA001
            else:
                crc >>= 1
    return crc & 0xFFFF
```

---

## 4. Key Protocol Command IDs

Extracted from `com.nothing.base.protocol.constant.ProtocolConstant`:

### 4.1 Asynchronous Notifications (`0xE0xx`)
| ID (Hex) | Name | Description |
|---|---|---|
| `0xE001` | `EVENT_BATTERY_CHANGED` | Real-time battery status for Left, Right, and Case |
| `0xE002` | `EVENT_DEVICE_STATUS_CHANGED` | Connect, disconnect, wearing state |
| `0xE003` | `EVENT_NOISE_REDUCTION_LEVEL_CHANGED` | ANC mode / level transitions |
| `0xE005` | `EVENT_GAME_MODE_CHANGED` | Low-latency state toggle |
| `0xE00E` | `EVENT_DUAL_DEVICE_CONNECT_STATE` | Dual-connection device info |
| `0xE014` | **`EVENT_MAGIC_BUTTON`** | **Case Smart Dial rotation and click events!** |
| `0xE015` | `EVENT_HEAD_TRACK` | Spatial audio head tracking updates |

### 4.2 Query Commands (`0xC0xx`)
| ID (Hex) | Name | Description |
|---|---|---|
| `0xC001` | `GET_PROTOCOL_VERSION` | Handshake; check firmware protocol compatibility |
| `0xC006` | `GET_REMOTE_CONFIGURATION` | Reads serial number and hardware SKU |
| `0xC007` | `GET_REMOTE_BATTERY_LEVEL` | Poll current battery levels |
| `0xC00D` | `GET_SUPPORTED_FEATURE` | Bitmask of supported firmware capabilities |
| `0xC018` | `GET_KEY_CONFIGURATION` | Read current gesture/button configuration |
| `0xC01C` | `GET_DEVICE_MODEL` | Device model identifier (`B172`) |
| `0xC01E` | `GET_CURRENT_NOISE_REDUCTION` | Active ANC mode |
| `0xC01F` | `GET_EQ_MODE` | Active EQ or Listening Mode preset |
| `0xC042` | `GET_HOST_VERSION_DEVICE` | Firmware version string |
| `0xC05C` | `GET_BOX_VERSION` | Charging case firmware version |

### 4.3 Set Commands (`0xF0xx`)
| ID (Hex) | Name | Description |
|---|---|---|
| `0xF001` | `SET_PROTOCOL_ACTIVATED` | Handshake completion ack (activates control session) |
| `0xF003` | `SET_KEY_CONFIGURATION` | Configure button/dial gestures on the hardware |
| `0xF00F` | `SET_CURRENT_NOISE_REDUCTION` | Set ANC mode (`0x01` High, `0x02` Mid, `0x05` Off, `0x07` Trans) |
| `0xF01D` | `SET_LISTENING_MODE` | Set listening mode preset for B172 |
| `0xF041` | `SET_CUSTOM_EQ` | Write 3-band parametric EQ curve to device |

---

## 5. Smart Dial (Magic Button) Specification

Extracted from `com.nothing.earbase.control.entity.ControlConfigurationEntity`:

### 5.1 Device Target (`deviceType`)
- `1`: Left Earbud
- `2`: Right Earbud
- `4`: **Case Dial (Smart Dial)**

### 5.2 Physical Button ID (`button`)
- `1`: Dial Center Click / Function Button
- `2`: Volume Up (mapped)
- `3`: Volume Down (mapped)
- `4`: ANC Button
- `5`: Rotate Clockwise (`BUTTON_SWIPE_UP`)
- `6`: Rotate Counter-Clockwise (`BUTTON_SWIPE_DOWN`)

### 5.3 Dial Gesture Constants (`gesture`)
| Value | Constant Name | Description |
|---|---|---|
| `1` | `GESTURE_PRESS` (`single_press`) | Single click on the dial |
| `2` | `GESTURE_DOUBLE_PRESS` | Double click on the dial |
| `3` | `GESTURE_TRIPLE_PRESS` | Triple click on the dial |
| `7` | `GESTURE_LONG_PRESS` (`press_hold`) | Press and hold on the dial |
| `10` | `GESTURE_ROTATE` (`rotate`) | Spinning the dial knob |
| `2` (fixed) | `double_press_hold` | Case Lock / Unlock toggle |

### 5.4 Operations Table (`operation`)
| Value | Constant Name | Function |
|---|---|---|
| `1` | `OPERATION_CLOSE` | No Action / Disabled |
| `2` | `OPERATION_PLAY_OR_PAUSE` | Toggle media playback |
| `6` | `OPERATION_VOLUME_UP` | Volume Up |
| `7` | `OPERATION_VOLUME_DOWN` | Volume Down |
| `8` | `OPERATION_PREVIOUS` | Skip Back |
| `9` | `OPERATION_NEXT` | Skip Forward |
| `10` | `OPERATION_ANC_CTRL` | Cycle ANC modes |
| `11` | `OPERATION_VOICE_ASSISTANT` | Launch voice assistant |
| `17` | `OPERATION_GAME_MODE` | Low latency game mode |
| `23` | **`OPERATION_ROTATE_CONTROL`** | **Dial Rotation (Volume / Scrubbing)** |
| `25` | `OPERATION_MIC_MUTE` | Mute microphone in call |
| `26` | `OPERATION_ANSWER_DECLINE_CALL` | Call control |
| `31` | `OPERATION_NEWS_WIDGET` | Nothing News Widget trigger |
| `34` | `OPERATION_EQ_PRESET` | Cycle EQ presets |
| `40` | `OPERATION_CASE_LOCK` | Lock case dial |

---

## 6. Real-Time Dial Event (`0xE014`) Payload Structure

When the dial is manipulated, the earbuds push an `0xE014` (`EVENT_MAGIC_BUTTON`) packet over the RFCOMM link:

```
[0x55] [0x00] [0x01] [0x14] [0xE0] [0x02] [0x00] [FSN] [Event_Type] [Value]
```

- **Command**: `0x14 0xE0` (`0xE014`)
- **Flags**: Typically `0x00` (unsolicited asynchronous device notification, no trailing CRC) or `0x60` (with CRC)
- **Payload**:
  - Byte 0: `Event_Type` / Gesture code (`0x01` = Single Click, `0x02` = Double Click, `0x07` = Hold, `0x17` or `0x0A` = Rotation)
  - Byte 1: `Value` / Direction / Step count (for rotation: `+1` clockwise vs `-1` / `0xFF` counter-clockwise)

---

## 7. Handshake Sequence

To receive continuous dial events and read device state without disconnection:

1. **Open RFCOMM Socket**: Connect to the device address on its active RFCOMM channel.
2. **Send Protocol Version Query**:
   ```
   55 60 01 01 C0 00 00 01 [CRC-16]
   ```
3. **Receive Handshake Ack**:
   ```
   55 60 01 01 40 [Len] [FSN] [Version Payload...] [CRC-16]
   ```
4. **Send Protocol Activated Ack**:
   ```
   55 60 01 01 F0 00 00 02 [CRC-16]
   ```
5. **Listen**: Maintain socket open. Dial turns and clicks stream asynchronously as `0xE014` packets!
