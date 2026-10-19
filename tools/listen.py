#!/usr/bin/env python3
"""Live Dial Event Monitor for CMF Buds Pro 2.

Connects to your CMF Buds Pro 2 over RFCOMM, completes the handshake,
and streams decoded dial rotations and clicks in real time.
"""

import sys
import time
import socket
import argparse
from proto import (
    build_frame,
    parse_frame,
    connect_rfcomm,
    CMD_GET_PROTO_VERSION,
    CMD_SET_ACTIVATED,
    EVT_MAGIC_BUTTON,
    EVT_BATTERY,
    EVT_ANC_CHANGED,
    GESTURE_SINGLE_PRESS,
    GESTURE_DOUBLE_PRESS,
    GESTURE_TRIPLE_PRESS,
    GESTURE_PRESS_HOLD,
    GESTURE_ROTATE,
)

DEFAULT_MAC = "2C:BE:EE:31:11:77"
DEFAULT_CH = 15  # Fallback to 15 or 28


def decode_dial_event(payload: bytes):
    """Decode EVENT_MAGIC_BUTTON (0xE014) payload."""
    if not payload:
        return "Empty Dial Payload"

    hex_str = payload.hex(" ")
    out = f"Payload: [{hex_str}] -> "

    # Check for rotation or gesture code
    code = payload[0]
    if len(payload) >= 2:
        val = payload[1]
        if code == GESTURE_ROTATE or code == 0x17 or code == 0x0A:
            direction = "CLOCKWISE (+)" if val < 128 else "COUNTER-CLOCKWISE (-)"
            return out + f"DIAL ROTATE: {direction} (step: {val})"
        elif code == GESTURE_SINGLE_PRESS:
            return out + "DIAL SINGLE CLICK"
        elif code == GESTURE_DOUBLE_PRESS:
            return out + "DIAL DOUBLE CLICK"
        elif code == GESTURE_TRIPLE_PRESS:
            return out + "DIAL TRIPLE CLICK"
        elif code == GESTURE_PRESS_HOLD:
            return out + "DIAL LONG PRESS / HOLD"

    if code == 1:
        return out + "DIAL CLICK (Action 1)"
    elif code == 2:
        return out + "DIAL DOUBLE CLICK / TOGGLE"
    elif code == 11:
        return out + "DIAL ACTION: VOICE ASSISTANT (0x0B)"

    return out + f"Raw Dial Code: 0x{code:02X}"


def main():
    parser = argparse.ArgumentParser(description="Listen to CMF Buds Pro 2 Smart Dial events")
    parser.add_argument("--mac", default=DEFAULT_MAC, help=f"Bluetooth MAC address (default: {DEFAULT_MAC})")
    parser.add_argument("--channel", type=int, default=DEFAULT_CH, help=f"RFCOMM channel (default: {DEFAULT_CH})")
    args = parser.parse_args()

    print("==================================================")
    print(f"Connecting to CMF Buds Pro 2 ({args.mac}) on RFCOMM ch {args.channel}...")
    print("==================================================")

    try:
        sock = connect_rfcomm(args.mac, args.channel, timeout=5.0)
    except Exception as e:
        print(f"Error connecting: {e}")
        print("Tip: Run ./probe.py first to find the active RFCOMM channel.")
        sys.exit(1)

    print("[+] Connected! Sending handshake...")
    try:
        # Handshake: GET_PROTOCOL_VERSION
        sock.sendall(build_frame(CMD_GET_PROTO_VERSION, fsn=1))
        # Activation: SET_PROTOCOL_ACTIVATED
        time.sleep(0.1)
        sock.sendall(build_frame(CMD_SET_ACTIVATED, fsn=2))
    except Exception as e:
        print(f"Handshake error: {e}")

    print("\n[★] LISTENING FOR DIAL EVENTS... (Turn or click the dial on your case!)")
    print("Press Ctrl+C to exit.\n")

    sock.settimeout(None)  # Blocking recv
    stream_buf = b""

    try:
        while True:
            chunk = sock.recv(1024)
            if not chunk:
                print("[-] Connection closed by device.")
                break

            stream_buf += chunk
            while True:
                frame, stream_buf = parse_frame(stream_buf)
                if not frame:
                    break

                cmd = frame["cmd"]
                name = frame["name"]
                payload = frame["payload"]

                if cmd == EVT_MAGIC_BUTTON:
                    desc = decode_dial_event(payload)
                    print(f"\033[92m[DIAL EVENT]\033[0m {desc}")
                elif cmd == EVT_BATTERY:
                    # Battery triplet [type, val]
                    if len(payload) >= 2:
                        print(f"[BATTERY] {payload.hex(' ')}")
                elif cmd == EVT_ANC_CHANGED:
                    print(f"[ANC MODE] {payload.hex(' ')}")
                else:
                    print(f"[PACKET] 0x{cmd:04X} ({name}) | Len {len(payload)}: {payload.hex(' ')}")

    except KeyboardInterrupt:
        print("\nExiting listener.")
    finally:
        sock.close()


if __name__ == "__main__":
    main()
