#!/usr/bin/env python3
"""CMF Buds Pro 2 (B172) Protocol Engine.

Implements the 0x55 RFCOMM framing, CRC-16/MODBUS verification,
command packet generation, and asynchronous event decoding.
"""

import socket
import struct
import time
from typing import Optional, Tuple, Dict, Any, Callable

# Protocol Constants
SYNC = 0x55
FLAG_CRC = 0x60
FLAG_NO_CRC = 0x00
HDR_LEN = 8  # sync(1) + flags(1) + target(1) + cmd(2) + len(2) + fsn(1)
CRC_LEN = 2

# Commands
CMD_GET_PROTO_VERSION = 0xC001
CMD_GET_DEVICE_MODEL  = 0xC01C
CMD_GET_BATTERY       = 0xC007
CMD_GET_ANC           = 0xC01E
CMD_GET_KEY_CONFIG    = 0xC018
CMD_SET_ACTIVATED     = 0xF001
CMD_SET_KEY_CONFIG    = 0xF003
CMD_SET_ANC           = 0xF00F

# Asynchronous Events
EVT_BATTERY           = 0xE001
EVT_STATUS            = 0xE002
EVT_ANC_CHANGED       = 0xE003
EVT_GAME_MODE         = 0xE005
EVT_MAGIC_BUTTON      = 0xE014  # Dial clicks and rotations!

# Smart Dial Gestures
GESTURE_SINGLE_PRESS  = 1
GESTURE_DOUBLE_PRESS  = 2
GESTURE_TRIPLE_PRESS  = 3
GESTURE_PRESS_HOLD    = 7
GESTURE_ROTATE        = 10

COMMAND_NAMES = {
    0xC001: "GET_PROTOCOL_VERSION",
    0xC006: "GET_REMOTE_CONFIG",
    0xC007: "GET_BATTERY_LEVEL",
    0xC018: "GET_KEY_CONFIGURATION",
    0xC01C: "GET_DEVICE_MODEL",
    0xC01E: "GET_CURRENT_NOISE_REDUCTION",
    0xC01F: "GET_EQ_MODE",
    0xC042: "GET_FIRMWARE_VERSION",
    0xC05C: "GET_BOX_VERSION",
    0xF001: "SET_PROTOCOL_ACTIVATED",
    0xF003: "SET_KEY_CONFIGURATION",
    0xF00F: "SET_CURRENT_NOISE_REDUCTION",
    0xE001: "EVENT_BATTERY_CHANGED",
    0xE002: "EVENT_DEVICE_STATUS_CHANGED",
    0xE003: "EVENT_NOISE_REDUCTION_CHANGED",
    0xE005: "EVENT_GAME_MODE_CHANGED",
    0xE014: "EVENT_MAGIC_BUTTON",
    0xE015: "EVENT_HEAD_TRACK",
}


def crc16_modbus(data: bytes) -> int:
    """Calculate CRC-16/MODBUS (init 0xFFFF, poly 0xA001 reflected)."""
    crc = 0xFFFF
    for b in data:
        crc ^= b
        for _ in range(8):
            crc = (crc >> 1) ^ 0xA001 if (crc & 1) else (crc >> 1)
    return crc & 0xFFFF


def build_frame(cmd: int, payload: bytes = b"", fsn: int = 1, flags: int = FLAG_CRC, target: int = 0x01) -> bytes:
    """Pack an outgoing command frame with optional CRC."""
    body = (
        bytes([SYNC, flags, target])
        + struct.pack("<H", cmd)
        + struct.pack("<H", len(payload))
        + bytes([fsn & 0xFF])
        + payload
    )
    if flags & 0x20:
        crc = crc16_modbus(body)
        return body + struct.pack("<H", crc)
    return body


def parse_frame(buf: bytes) -> Tuple[Optional[Dict[str, Any]], bytes]:
    """Parse one frame from a stream buffer.
    
    Returns (frame_dict, remaining_buffer).
    """
    if len(buf) < HDR_LEN:
        return None, buf

    # Search for SOF 0x55
    sof_idx = buf.find(bytes([SYNC]))
    if sof_idx == -1:
        return None, b""
    if sof_idx > 0:
        buf = buf[sof_idx:]

    if len(buf) < HDR_LEN:
        return None, buf

    flags = buf[1]
    target = buf[2]
    cmd = struct.unpack("<H", buf[3:5])[0]
    payload_len = struct.unpack("<H", buf[5:7])[0]
    fsn = buf[7]

    has_crc = bool(flags & 0x20)
    total_len = HDR_LEN + payload_len + (CRC_LEN if has_crc else 0)

    if len(buf) < total_len:
        return None, buf  # Wait for more bytes

    payload = buf[HDR_LEN : HDR_LEN + payload_len]

    if has_crc:
        expected_crc = struct.unpack("<H", buf[HDR_LEN + payload_len : total_len])[0]
        actual_crc = crc16_modbus(buf[: HDR_LEN + payload_len])
        if actual_crc != expected_crc:
            # Bad CRC, skip this SOF and continue
            return None, buf[1:]

    cmd_name = COMMAND_NAMES.get(cmd)
    if not cmd_name:
        # Check if it's a response with bit 15 cleared
        req_cmd = cmd | 0x8000
        cmd_name = COMMAND_NAMES.get(req_cmd, f"UNKNOWN_0x{cmd:04X}") + " (RESP)"

    frame = {
        "cmd": cmd,
        "name": cmd_name,
        "flags": flags,
        "target": target,
        "fsn": fsn,
        "payload": payload,
        "raw": buf[:total_len],
    }

    return frame, buf[total_len:]


def connect_rfcomm(mac: str, channel: int, timeout: float = 3.0) -> socket.socket:
    """Open an RFCOMM connection to the target device."""
    sock = socket.socket(socket.AF_BLUETOOTH, socket.SOCK_STREAM, socket.BTPROTO_RFCOMM)
    sock.settimeout(timeout)
    sock.connect((mac, channel))
    return sock
