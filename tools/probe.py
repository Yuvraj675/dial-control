#!/usr/bin/env python3
"""Channel scanner and handshake prober for CMF Buds Pro 2.

Scans RFCOMM channels (default 1 to 30) on your paired CMF Buds Pro 2,
sends a GET_PROTOCOL_VERSION handshake probe, and prints responsive channels.
"""

import sys
import time
import socket
import argparse
from proto import (
    build_frame,
    parse_frame,
    CMD_GET_PROTO_VERSION,
    CMD_SET_ACTIVATED,
    connect_rfcomm,
)

DEFAULT_MAC = "2C:BE:EE:31:11:77"
# Channels commonly used across Nothing & CMF hardware:
# Ear (1, 2, a): ch 15
# CMF Headphone Pro: ch 28
# Status/Legacy: ch 17
PRIORITY_CHANNELS = [15, 28, 17, 12, 13, 14, 16, 18, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]


def probe_channel(mac: str, channel: int, timeout: float = 1.5) -> bool:
    try:
        sock = connect_rfcomm(mac, channel, timeout=timeout)
    except (socket.error, OSError) as e:
        return False

    print(f"  [+] Connected to RFCOMM channel {channel}!")
    try:
        # Send GET_PROTOCOL_VERSION
        probe_packet = build_frame(CMD_GET_PROTO_VERSION, payload=b"", fsn=1)
        sock.sendall(probe_packet)

        # Wait for reply
        sock.settimeout(1.5)
        resp = sock.recv(1024)
        if resp:
            frame, _ = parse_frame(resp)
            if frame:
                print(f"  [✓] Received valid response on channel {channel}!")
                print(f"      Command: 0x{frame['cmd']:04X} ({frame['name']})")
                print(f"      Payload: {frame['payload'].hex()}")
                sock.close()
                return True
            else:
                print(f"  [?] Received raw data on ch {channel}: {resp.hex()}")
        sock.close()
    except Exception as e:
        print(f"  [-] Error while querying ch {channel}: {e}")
        try:
            sock.close()
        except:
            pass

    return False


def main():
    parser = argparse.ArgumentParser(description="Probe CMF Buds Pro 2 RFCOMM channels")
    parser.add_argument("--mac", default=DEFAULT_MAC, help=f"Bluetooth MAC address (default: {DEFAULT_MAC})")
    parser.add_argument("--full", action="store_true", help="Scan all channels from 1 to 30")
    args = parser.parse_args()

    channels = list(range(1, 31)) if args.full else PRIORITY_CHANNELS
    print(f"==================================================")
    print(f"Probing CMF Buds Pro 2 ({args.mac})")
    print(f"Ensure earbuds are out of the case or case is open & connected")
    print(f"==================================================")

    found = []
    for ch in channels:
        print(f"Testing channel {ch:2d}...", end="\r", flush=True)
        if probe_channel(args.mac, ch):
            found.append(ch)

    print("\n--------------------------------------------------")
    if found:
        print(f"SUCCESS: Active Nothing protocol channel(s): {found}")
        print(f"Use: ./listen.py --channel {found[0]}")
    else:
        print("No active response found.")
        print("Tips:")
        print("  1. Make sure your earbuds are connected in Bluetooth settings (`bluetoothctl connect 2C:BE:EE:31:11:77`)")
        print("  2. Disconnect them from your phone so the RFCOMM link is free.")
        print("  3. Run with `--full` to scan all 30 channels.")


if __name__ == "__main__":
    main()
