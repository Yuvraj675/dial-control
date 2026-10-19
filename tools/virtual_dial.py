#!/usr/bin/env python3
"""Virtual Radial Dial UI & Daemon for CMF Buds Pro 2.

Features:
- On-screen radial dial with CMF aesthetic
- Mouse wheel / arrow keys to rotate
- Left click / Enter to select & execute
- Right click / Esc / 'X' button to dismiss
- Auto-focus so keyboard shortcuts work reliably
"""

import math
import sys
import os
import threading
import subprocess
import tkinter as tk
from typing import List, Tuple

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


class VirtualDialWindow(tk.Tk):
    def __init__(self, mac="2C:BE:EE:31:11:77", channel=15, test_mode=False):
        super().__init__()
        self.mac = mac
        self.channel = channel
        self.test_mode = test_mode

        self.selected_index = 0
        self.size = 380
        self.radius = 145
        self.center_x = self.size // 2
        self.center_y = self.size // 2

        # Configure window
        self.title("CMF Smart Dial")
        self.overrideredirect(True)       # Frameless
        self.attributes("-topmost", True) # Always on top
        self.configure(bg="#141414")

        # Center on screen
        screen_w = self.winfo_screenwidth()
        screen_h = self.winfo_screenheight()
        pos_x = (screen_w - self.size) // 2
        pos_y = (screen_h - self.size) // 2
        self.geometry(f"{self.size}x{self.size}+{pos_x}+{pos_y}")

        # Canvas
        self.canvas = tk.Canvas(
            self,
            width=self.size,
            height=self.size,
            bg="#181818",
            highlightthickness=2,
            highlightbackground="#ff4500",  # CMF Signature Orange
        )
        self.canvas.pack(fill=tk.BOTH, expand=True)

        # Bind controls
        self.bind("<Escape>", lambda e: self.dismiss())
        self.bind("<Return>", lambda e: self.select_current())
        self.bind("<space>", lambda e: self.select_current())
        self.bind("<Right>", lambda e: self.rotate_cw())
        self.bind("<Down>", lambda e: self.rotate_cw())
        self.bind("<Left>", lambda e: self.rotate_ccw())
        self.bind("<Up>", lambda e: self.rotate_ccw())

        # Mouse controls: Scroll wheel rotates, Right click dismisses, Left click selects
        self.canvas.bind("<Button-4>", lambda e: self.rotate_ccw())   # Linux Scroll Up
        self.canvas.bind("<Button-5>", lambda e: self.rotate_cw())    # Linux Scroll Down
        self.canvas.bind("<MouseWheel>", self.on_mousewheel)
        self.canvas.bind("<Button-3>", lambda e: self.dismiss())      # Right click = close
        self.canvas.bind("<Button-1>", self.on_canvas_click)          # Left click

        self.draw_dial()

        # Grab focus so keyboard commands work instantly
        self.after(50, self.force_focus)

        # Start Bluetooth worker if not test mode
        if not self.test_mode:
            self.bt_thread = threading.Thread(target=self.bluetooth_worker, daemon=True)
            self.bt_thread.start()

    def force_focus(self):
        self.lift()
        self.focus_force()

    def on_mousewheel(self, event):
        if event.delta > 0:
            self.rotate_ccw()
        else:
            self.rotate_cw()

    def on_canvas_click(self, event):
        # Click close button at top-right
        if event.x > self.size - 40 and event.y < 40:
            self.dismiss()
            return

        # Click inside center knob -> confirm
        dx = event.x - self.center_x
        dy = event.y - self.center_y
        dist = math.sqrt(dx * dx + dy * dy)
        if dist <= 65:
            self.select_current()
            return

        # Click any option node
        n = len(DIAL_ACTIONS)
        angle_step = 2 * math.pi / n
        for i in range(n):
            angle = i * angle_step - (math.pi / 2)
            nx = self.center_x + (self.radius - 30) * math.cos(angle)
            ny = self.center_y + (self.radius - 30) * math.sin(angle)
            if math.sqrt((event.x - nx)**2 + (event.y - ny)**2) <= 25:
                self.selected_index = i
                self.draw_dial()
                self.select_current()
                return

    def draw_dial(self):
        self.canvas.delete("all")
        n = len(DIAL_ACTIONS)
        angle_step = 2 * math.pi / n

        # Close button in top-right corner
        self.canvas.create_text(
            self.size - 22, 22,
            text="✕",
            fill="#888888",
            font=("Sans", 14, "bold")
        )

        # Outer track ring
        self.canvas.create_oval(
            self.center_x - self.radius,
            self.center_y - self.radius,
            self.center_x + self.radius,
            self.center_y + self.radius,
            outline="#282828",
            width=6,
        )

        # Nodes
        for i, (label, _) in enumerate(DIAL_ACTIONS):
            angle = i * angle_step - (math.pi / 2)
            x = self.center_x + (self.radius - 30) * math.cos(angle)
            y = self.center_y + (self.radius - 30) * math.sin(angle)

            is_selected = (i == self.selected_index)
            fill_color = "#ff4500" if is_selected else "#222222"
            outline_color = "#ffffff" if is_selected else "#444444"
            r = 22 if is_selected else 16

            self.canvas.create_oval(
                x - r, y - r, x + r, y + r,
                fill=fill_color,
                outline=outline_color,
                width=2 if is_selected else 1,
            )

        # Center knob
        active_label, _ = DIAL_ACTIONS[self.selected_index]
        self.canvas.create_oval(
            self.center_x - 62,
            self.center_y - 62,
            self.center_x + 62,
            self.center_y + 62,
            fill="#1e1e1e",
            outline="#ff4500",
            width=3,
        )
        self.canvas.create_text(
            self.center_x,
            self.center_y - 8,
            text=active_label,
            fill="#ffffff",
            font=("Sans", 10, "bold"),
            justify=tk.CENTER,
        )
        self.canvas.create_text(
            self.center_x,
            self.center_y + 18,
            text="Click / Enter to Select\nEsc to Close",
            fill="#777777",
            font=("Sans", 7),
            justify=tk.CENTER,
        )

    def rotate_cw(self):
        self.selected_index = (self.selected_index + 1) % len(DIAL_ACTIONS)
        self.draw_dial()

    def rotate_ccw(self):
        self.selected_index = (self.selected_index - 1) % len(DIAL_ACTIONS)
        self.draw_dial()

    def select_current(self):
        _, action_key = DIAL_ACTIONS[self.selected_index]
        self.execute_action(action_key)
        self.dismiss()

    def dismiss(self):
        self.destroy()

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
                sock.sendall(build_frame(CMD_GET_PROTO_VERSION, fsn=1))
                sock.sendall(build_frame(CMD_SET_ACTIVATED, fsn=2))
                print("[BT] Connected and listening for dial events!")

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
                                direction = 1 if val < 128 else -1
                                if direction > 0:
                                    self.after(0, self.rotate_cw)
                                else:
                                    self.after(0, self.rotate_ccw)
                            elif code in (GESTURE_SINGLE_PRESS, 1, GESTURE_DOUBLE_PRESS, 2):
                                self.after(0, self.select_current)
            except Exception as e:
                print(f"[BT] Connection dropped/error: {e}")
                import time
                time.sleep(3.0)


if __name__ == "__main__":
    import argparse
    parser = argparse.ArgumentParser(description="CMF Buds Pro 2 Virtual Dial UI")
    parser.add_argument("--test", action="store_true", help="Run standalone test UI")
    parser.add_argument("--mac", default="2C:BE:EE:31:11:77")
    parser.add_argument("--channel", type=int, default=15)
    args = parser.parse_args()

    app = VirtualDialWindow(mac=args.mac, channel=args.channel, test_mode=args.test)
    app.mainloop()
