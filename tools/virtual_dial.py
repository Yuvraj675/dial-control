#!/usr/bin/env python3
"""Virtual Radial Dial UI & Daemon for CMF Buds Pro 2.

When you click the case dial, this pops up a smooth on-screen radial menu.
Rotating the dial rotates the selection.
Clicking again executes the selected action and dismisses the dial.

Can run with:
  1. Live Bluetooth RFCOMM mode: ./virtual_dial.py --mac 2C:BE:EE:31:11:77 --channel 15
  2. Test mode (Keyboard arrows & Enter): ./virtual_dial.py --test
"""

import math
import sys
import os
import threading
import subprocess
import tkinter as tk
from typing import List, Tuple

# Options displayed in the virtual dial
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


class RadialMenu(tk.Toplevel):
    def __init__(self, parent, on_select_callback):
        super().__init__(parent)
        self.on_select = on_select_callback
        self.selected_index = 0
        self.radius = 160
        self.center_x = 200
        self.center_y = 200
        self.size = 400

        # Window settings
        self.overrideredirect(True)  # Frameless
        self.attributes("-topmost", True)  # Always on top
        self.configure(bg="#121212")

        # Center on screen
        screen_w = self.winfo_screenwidth()
        screen_h = self.winfo_screenheight()
        pos_x = (screen_w - self.size) // 2
        pos_y = (screen_h - self.size) // 2
        self.geometry(f"{self.size}x{self.size}+{pos_x}+{pos_y}")

        # Canvas for radial drawing
        self.canvas = tk.Canvas(
            self,
            width=self.size,
            height=self.size,
            bg="#181818",
            highlightthickness=2,
            highlightbackground="#ff4500",  # CMF Signature Orange accent!
        )
        self.canvas.pack(fill=tk.BOTH, expand=True)

        self.draw_dial()

    def draw_dial(self):
        self.canvas.delete("all")
        n = len(DIAL_ACTIONS)
        angle_step = 2 * math.pi / n

        # Outer track ring
        self.canvas.create_oval(
            self.center_x - self.radius,
            self.center_y - self.radius,
            self.center_x + self.radius,
            self.center_y + self.radius,
            outline="#2a2a2a",
            width=8,
        )

        # Draw items
        for i, (label, _) in enumerate(DIAL_ACTIONS):
            angle = i * angle_step - (math.pi / 2)  # Start at 12 o'clock
            x = self.center_x + (self.radius - 35) * math.cos(angle)
            y = self.center_y + (self.radius - 35) * math.sin(angle)

            is_selected = i == self.selected_index
            bg_color = "#ff4500" if is_selected else "#282828"
            fg_color = "#ffffff" if is_selected else "#aaaaaa"
            font_weight = "bold" if is_selected else "normal"

            # Node circle
            r_node = 24 if is_selected else 18
            self.canvas.create_oval(
                x - r_node,
                y - r_node,
                x + r_node,
                y + r_node,
                fill=bg_color,
                outline="#ffffff" if is_selected else "#444444",
                width=2 if is_selected else 1,
            )

        # Center knob with active action label
        active_label, _ = DIAL_ACTIONS[self.selected_index]
        self.canvas.create_oval(
            self.center_x - 70,
            self.center_y - 70,
            self.center_x + 70,
            self.center_y + 70,
            fill="#222222",
            outline="#ff4500",
            width=3,
        )
        self.canvas.create_text(
            self.center_x,
            self.center_y - 10,
            text=active_label,
            fill="#ffffff",
            font=("Sans", 11, "bold"),
            justify=tk.CENTER,
        )
        self.canvas.create_text(
            self.center_x,
            self.center_y + 20,
            text="Click to Select",
            fill="#888888",
            font=("Sans", 8),
        )

    def rotate_cw(self):
        self.selected_index = (self.selected_index + 1) % len(DIAL_ACTIONS)
        self.draw_dial()

    def rotate_ccw(self):
        self.selected_index = (self.selected_index - 1) % len(DIAL_ACTIONS)
        self.draw_dial()

    def select(self):
        _, action_key = DIAL_ACTIONS[self.selected_index]
        self.on_select(action_key)
        self.destroy()


class VirtualDialApp:
    def __init__(self, mac="2C:BE:EE:31:11:77", channel=15, test_mode=False):
        self.mac = mac
        self.channel = channel
        self.test_mode = test_mode
        self.menu: RadialMenu = None

        # Hidden root
        self.root = tk.Tk()
        self.root.withdraw()

        if self.test_mode:
            print("==================================================")
            print("Virtual Dial TEST MODE:")
            print("  Space / Tab : Open / Toggle Dial")
            print("  Up / Right  : Rotate Clockwise")
            print("  Down / Left : Rotate Counter-Clockwise")
            print("  Enter       : Select & Confirm Action")
            print("  Esc         : Dismiss Dial")
            print("==================================================")
            self.root.bind("<space>", lambda e: self.toggle_dial())
            self.root.bind("<Right>", lambda e: self.on_rotate(1))
            self.root.bind("<Left>", lambda e: self.on_rotate(-1))
            self.root.bind("<Return>", lambda e: self.on_confirm())
            self.root.bind("<Escape>", lambda e: self.close_dial())
            # Open immediately in test mode
            self.toggle_dial()
        else:
            # Start background Bluetooth worker
            self.bt_thread = threading.Thread(target=self.bluetooth_worker, daemon=True)
            self.bt_thread.start()

    def toggle_dial(self):
        if self.menu and self.menu.winfo_exists():
            self.close_dial()
        else:
            self.menu = RadialMenu(self.root, self.execute_action)

    def close_dial(self):
        if self.menu and self.menu.winfo_exists():
            self.menu.destroy()
            self.menu = None

    def on_rotate(self, direction: int):
        if self.menu and self.menu.winfo_exists():
            if direction > 0:
                self.menu.rotate_cw()
            else:
                self.menu.rotate_ccw()

    def on_confirm(self):
        if self.menu and self.menu.winfo_exists():
            self.menu.select()
            self.menu = None

    def execute_action(self, action_key: str):
        print(f"[ACTION TRIGGERED] -> {action_key}")
        try:
            if action_key == "volume":
                subprocess.run(["pactl", "set-sink-volume", "@DEFAULT_SINK@", "+5%"])
            elif action_key == "mute":
                subprocess.run(["pactl", "set-sink-mute", "@DEFAULT_SINK@", "toggle"])
            elif action_key == "media_play":
                subprocess.run(["playerctl", "play-pause"])
            elif action_key == "media_next":
                subprocess.run(["playerctl", "next"])
            elif action_key == "media_prev":
                subprocess.run(["playerctl", "previous"])
            elif action_key == "brightness":
                subprocess.run(["brightnessctl", "set", "+10%"])
        except Exception as e:
            print(f"Error executing action: {e}")

    def bluetooth_worker(self):
        from proto import (
            connect_rfcomm,
            build_frame,
            parse_frame,
            CMD_GET_PROTO_VERSION,
            CMD_SET_ACTIVATED,
            EVT_MAGIC_BUTTON,
            GESTURE_ROTATE,
            GESTURE_SINGLE_PRESS,
            GESTURE_DOUBLE_PRESS,
        )

        while True:
            try:
                print(f"[BT] Connecting to CMF Buds Pro 2 on ch {self.channel}...")
                sock = connect_rfcomm(self.mac, self.channel, timeout=5.0)
                # Handshake
                sock.sendall(build_frame(CMD_GET_PROTO_VERSION, fsn=1))
                sock.sendall(build_frame(CMD_SET_ACTIVATED, fsn=2))
                print("[BT] Connected and activated!")

                buf = b""
                sock.settimeout(None)
                while True:
                    chunk = sock.recv(1024)
                    if not chunk:
                        break
                    buf += chunk
                    while True:
                        frame, buf = parse_frame(buf)
                        if not frame:
                            break
                        if frame["cmd"] == EVT_MAGIC_BUTTON:
                            payload = frame["payload"]
                            code = payload[0] if payload else 0
                            val = payload[1] if len(payload) > 1 else 0

                            if code in (GESTURE_ROTATE, 0x17, 0x0A):
                                # Rotate dial
                                direction = 1 if val < 128 else -1
                                self.root.after(0, self.on_rotate, direction)
                            elif code in (GESTURE_SINGLE_PRESS, 1):
                                # Tap dial -> Toggle Menu or Confirm if already open
                                if self.menu and self.menu.winfo_exists():
                                    self.root.after(0, self.on_confirm)
                                else:
                                    self.root.after(0, self.toggle_dial)
                            elif code in (GESTURE_DOUBLE_PRESS, 2):
                                self.root.after(0, self.on_confirm)
            except Exception as e:
                print(f"[BT] Connection failed or dropped: {e}")
                import time
                time.sleep(3.0)

    def run(self):
        self.root.mainloop()


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser()
    parser.add_argument("--test", action="store_true", help="Run in test mode with keyboard controls")
    parser.add_argument("--mac", default="2C:BE:EE:31:11:77")
    parser.add_argument("--channel", type=int, default=15)
    args = parser.parse_args()

    app = VirtualDialApp(mac=args.mac, channel=args.channel, test_mode=args.test)
    app.run()
