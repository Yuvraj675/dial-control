#!/usr/bin/env python3
"""
Filee-Inspired Context-Aware Donut Radial HUD
Features:
- Segmented annular "donut" slices with smooth spring expansion
- Dynamic context-awareness (adapts to active window: Files, Media, Code, System)
- Center HUD badge displaying active app, action icon, title, and hotkey pill
- Native hotkey dispatch and system integration
"""

import sys
import os
import math
import time
import json
import tkinter as tk
from typing import Dict, Any, List, Optional

# Add project root to path
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
from tools.context_detector import ContextDetector


def get_donut_slice_points(cx: float, cy: float, r_in: float, r_out: float, a1: float, a2: float, steps: int = 24) -> List[float]:
    """Calculate points for a curved annular sector (donut slice)."""
    pts = []
    # Outer arc
    for i in range(steps + 1):
        t = a1 + (a2 - a1) * (i / steps)
        pts.extend([cx + r_out * math.cos(t), cy + r_out * math.sin(t)])
    # Inner arc (reversed)
    for i in range(steps + 1):
        t = a2 - (a2 - a1) * (i / steps)
        pts.extend([cx + r_in * math.cos(t), cy + r_in * math.sin(t)])
    return pts


class DonutRadialHUD(tk.Tk):
    def __init__(self, config_path: Optional[str] = None, on_action_cb = None, vk = None, on_mode_change_cb = None):
        super().__init__()

        self.on_action_cb = on_action_cb
        self.vk = vk
        self.on_mode_change_cb = on_mode_change_cb

        # Load Configuration & Context Detector
        if not config_path:
            config_path = os.path.join(os.path.dirname(__file__), "..", "config", "profiles.json")
        self.config_path = os.path.abspath(config_path)
        self.load_config()
        self.detector = ContextDetector(self.config)

        # Window Geometry & Dimensions
        self.size = 440
        self.center_x = self.size // 2
        self.center_y = self.size // 2

        self.r_outer = 185.0
        self.r_inner = 115.0
        self.slice_gap = math.radians(2.5)  # Angular gap between sectors

        # Window styling
        self.overrideredirect(True)
        self.wm_attributes("-topmost", True)
        self.configure(bg="#0d0e12")

        # Center on Primary Screen
        self.update_idletasks()
        sw = self.winfo_screenwidth()
        sh = self.winfo_screenheight()
        pos_x = (sw - self.size) // 2
        pos_y = (sh - self.size) // 2
        self.geometry(f"{self.size}x{self.size}+{pos_x}+{pos_y}")

        # High-performance Canvas
        self.canvas = tk.Canvas(
            self,
            width=self.size,
            height=self.size,
            bg="#0d0e12",
            highlightthickness=0,
            bd=0
        )
        self.canvas.pack(fill="both", expand=True)

        # State Variables
        self.current_profile_id = "default"
        self.current_profile_data: Dict[str, Any] = {}
        self.actions: List[Dict[str, Any]] = []
        self.selected_index = 0
        self.target_angle = 0.0
        self.display_angle = 0.0

        # Mode State ("menu", "volume", "brightness")
        self.current_mode = "menu"
        self.active_gauge_val = 50.0  # Percentage for gauge (0-100)

        # Palette (Filee-inspired Dark Acrylic & Neon Accents)
        self.colors = {
            "bg": "#0d0e12",
            "slice_idle": "#1a1b24",
            "slice_idle_border": "#2b2d3d",
            "slice_active": "#00d2ff",         # Vibrant cyan
            "slice_active_border": "#ffffff",
            "slice_glow": "#005577",
            "text_primary": "#ffffff",
            "text_muted": "#8d90a5",
            "hub_bg": "#12131a",
            "hub_border": "#282a38",
            "badge_bg": "#1e202b",
            "gauge_track": "#20222f",
            "gauge_fill": "#00f0ff",
        }

        # Event Bindings
        self.bind("<Escape>", lambda e: self.hide_hud())
        self.bind("<Button-3>", lambda e: self.hide_hud())   # Right click closes
        self.canvas.bind("<Button-1>", self.on_canvas_click) # Mouse click

        # Testing with scroll wheel
        self.canvas.bind("<Button-4>", lambda e: self.on_rotate_event(-1))
        self.canvas.bind("<Button-5>", lambda e: self.on_rotate_event(1))
        self.bind("<Up>", lambda e: self.on_rotate_event(-1))
        self.bind("<Down>", lambda e: self.on_rotate_event(1))
        self.bind("<Return>", lambda e: self.on_click_event())

        # Start hidden
        self.withdraw()
        self.is_visible = False

        # Animation Loop (60 FPS smooth easing)
        self.animate_tick()

    def load_config(self):
        try:
            with open(self.config_path, "r", encoding="utf-8") as f:
                self.config = json.load(f)
        except Exception as e:
            print(f"[!] Error loading config: {e}")
            self.config = {"settings": {}, "profiles": {}}

    def refresh_context(self):
        """Query foreground app and update active profile."""
        app_name, title = self.detector.get_active_window_info()
        p_id, p_data = self.detector.match_profile(app_name, title)
        self.current_profile_id = p_id
        self.current_profile_data = p_data
        self.active_app_name = app_name or "System"
        self.actions = p_data.get("actions", [])
        if self.selected_index >= len(self.actions):
            self.selected_index = 0

    @property
    def active_val(self):
        return self.active_gauge_val

    @active_val.setter
    def active_val(self, val):
        self.active_gauge_val = float(val)

    def show_hud(self):
        self.load_config()
        self.refresh_context()
        self.current_mode = "menu"
        self.is_visible = True
        self.deiconify()
        self.lift()
        self.after(20, self.focus_force)
        self.draw_ui()
        if self.on_mode_change_cb:
            self.on_mode_change_cb(self.current_mode)

    def hide_hud(self):
        self.is_visible = False
        self.current_mode = "menu"
        self.withdraw()
        if self.on_mode_change_cb:
            self.on_mode_change_cb("closed")

    def toggle(self):
        if self.is_visible:
            self.hide_hud()
        else:
            self.show_hud()

    def on_rotate_event(self, direction: int):
        """Handle rotation (direction > 0: CW, direction < 0: CCW)."""
        if not self.is_visible:
            return

        if self.current_mode == "menu":
            n = len(self.actions)
            if n > 0:
                if direction > 0:
                    self.selected_index = (self.selected_index + 1) % n
                else:
                    self.selected_index = (self.selected_index - 1) % n
                self.draw_ui()

        elif self.current_mode == "brightness":
            if self.vk:
                if direction > 0:
                    self.vk.step_brightness_up()
                else:
                    self.vk.step_brightness_down()
            step = 5.0 if direction > 0 else -5.0
            self.active_gauge_val = max(0.0, min(100.0, self.active_gauge_val + step))
            self.draw_ui()

        elif self.current_mode == "volume":
            if self.on_action_cb:
                self.on_action_cb("volume", direction, self.active_gauge_val)

    def on_click_event(self):
        """Handle center knob click."""
        if not self.is_visible:
            self.show_hud()
            return

        if self.current_mode == "menu":
            if 0 <= self.selected_index < len(self.actions):
                action = self.actions[self.selected_index]
                self.execute_action(action)
        else:
            # Inside gauge mode (volume / brightness) -> click returns to menu or dismisses
            self.hide_hud()

    def execute_action(self, action: Dict[str, Any]):
        a_type = action.get("type")
        a_id = action.get("id")

        if a_type == "mode":
            self.current_mode = a_id.replace("_mode", "")
            if self.on_mode_change_cb:
                self.on_mode_change_cb(self.current_mode)
            self.draw_ui()
            return

        # Execute Hotkey
        if a_type == "hotkey" and self.vk:
            keys = action.get("keys", [])
            if hasattr(self.vk, "emit_hotkey_names"):
                self.vk.emit_hotkey_names(keys)
            self.hide_hud()
            return

        # Execute Media Key
        if a_type == "key" and self.vk:
            media_cmd = action.get("action")
            if media_cmd == "playpause":
                self.vk.play_pause()
            elif media_cmd == "next":
                self.vk.next_track()
            elif media_cmd == "prev":
                self.vk.prev_track()
            self.hide_hud()
            return

        # Execute Shell Command
        if a_type == "command":
            cmd = action.get("cmd")
            if cmd:
                os.system(f"nohup {cmd} >/dev/null 2>&1 &")
            self.hide_hud()
            return

        # Execute Mute
        if a_type == "mute" and self.on_action_cb:
            self.on_action_cb("mute_toggle", 0, 0)
            self.hide_hud()
            return

        # Generic callback
        if self.on_action_cb:
            self.on_action_cb(a_id, 0, 0)
        self.hide_hud()

    def animate_tick(self):
        """Spring easing animation tick."""
        if self.is_visible:
            # Subtle rotation interpolation if needed
            pass
        self.after(16, self.animate_tick)

    def draw_ui(self):
        self.canvas.delete("all")
        cx, cy = self.center_x, self.center_y

        # 1. Subtle Outer Ambient Glow Ring
        self.canvas.create_oval(
            cx - self.r_outer - 8, cy - self.r_outer - 8,
            cx + self.r_outer + 8, cy + self.r_outer + 8,
            outline="#161822", width=3
        )

        if self.current_mode == "menu":
            self.draw_donut_menu(cx, cy)
        else:
            self.draw_gauge_mode(cx, cy)

        # 2. Central Hub (Glass Floating Capsule)
        self.draw_center_hub(cx, cy)

    def draw_donut_menu(self, cx: float, cy: float):
        n = len(self.actions)
        if n == 0:
            return

        angle_step = (2 * math.pi) / n

        for i in range(n):
            action = self.actions[i]
            is_active = (i == self.selected_index)

            # Start and End Angles (starting at 12 o'clock = -pi/2)
            a_center = -math.pi / 2 + (i * angle_step)
            a1 = a_center - (angle_step / 2.0) + (self.slice_gap / 2.0)
            a2 = a_center + (angle_step / 2.0) - (self.slice_gap / 2.0)

            # Filee-Style Active Slice Pop (expands outward with luminous glow)
            if is_active:
                r_in = self.r_inner - 4
                r_out = self.r_outer + 9
                fill_color = self.colors["slice_active"]
                border_color = self.colors["slice_active_border"]
                border_width = 2
                glow_color = self.colors["slice_glow"]

                # Draw outer glow polygon
                glow_pts = get_donut_slice_points(cx, cy, r_in - 2, r_out + 4, a1, a2)
                self.canvas.create_polygon(glow_pts, fill=glow_color, outline="", smooth=False)
            else:
                r_in = self.r_inner
                r_out = self.r_outer
                fill_color = self.colors["slice_idle"]
                border_color = self.colors["slice_idle_border"]
                border_width = 1

            # Slice Polygon
            slice_pts = get_donut_slice_points(cx, cy, r_in, r_out, a1, a2)
            self.canvas.create_polygon(
                slice_pts,
                fill=fill_color,
                outline=border_color,
                width=border_width,
                smooth=False
            )

            # Slice Icon & Mini Label
            r_mid = (r_in + r_out) / 2.0
            icon_x = cx + r_mid * math.cos(a_center)
            icon_y = cy + r_mid * math.sin(a_center)

            icon = action.get("icon", "•")
            text_color = "#000000" if is_active else self.colors["text_primary"]

            self.canvas.create_text(
                icon_x, icon_y,
                text=icon,
                font=("Noto Color Emoji", 18, "bold") if "Emoji" in str(icon) else ("Inter", 18, "bold"),
                fill=text_color
            )

    def draw_gauge_mode(self, cx: float, cy: float):
        """Draw circular arc gauge for Volume / Brightness mode."""
        r_mid = (self.r_inner + self.r_outer) / 2.0
        gauge_width = 18

        # Background Track (360 degrees)
        self.canvas.create_oval(
            cx - r_mid, cy - r_mid,
            cx + r_mid, cy + r_mid,
            outline=self.colors["gauge_track"],
            width=gauge_width
        )

        # Active Fill Arc (-90 deg is 12 o'clock, clockwise extent)
        extent = -(self.active_gauge_val / 100.0) * 359.9
        self.canvas.create_arc(
            cx - r_mid, cy - r_mid,
            cx + r_mid, cy + r_mid,
            start=90,
            extent=extent,
            style="arc",
            outline=self.colors["gauge_fill"],
            width=gauge_width
        )

        # Indicator Knob at Tip
        tip_angle = -math.pi / 2 + math.radians((self.active_gauge_val / 100.0) * 360)
        knob_x = cx + r_mid * math.cos(tip_angle)
        knob_y = cy + r_mid * math.sin(tip_angle)
        self.canvas.create_oval(
            knob_x - 12, knob_y - 12,
            knob_x + 12, knob_y + 12,
            fill="#ffffff",
            outline=self.colors["gauge_fill"],
            width=3
        )

    def draw_center_hub(self, cx: float, cy: float):
        """Floating center capsule with context badge, active icon, and label."""
        hub_r = self.r_inner - 10

        # Hub Disc
        self.canvas.create_oval(
            cx - hub_r, cy - hub_r,
            cx + hub_r, cy + hub_r,
            fill=self.colors["hub_bg"],
            outline=self.colors["hub_border"],
            width=2
        )

        if self.current_mode == "menu":
            if 0 <= self.selected_index < len(self.actions):
                act = self.actions[self.selected_index]
                profile_name = self.current_profile_data.get("name", "System")
                profile_icon = self.current_profile_data.get("icon", "⚙️")

                # Top Category Badge Pill
                badge_w = 120
                badge_h = 24
                self.canvas.create_rectangle(
                    cx - badge_w // 2, cy - 65,
                    cx + badge_w // 2, cy - 41,
                    fill=self.colors["badge_bg"],
                    outline="#2d3040",
                    width=1
                )
                self.canvas.create_text(
                    cx, cy - 53,
                    text=f"{profile_icon} {profile_name}",
                    font=("Inter", 9, "bold"),
                    fill=self.colors["text_muted"]
                )

                # Center Action Large Icon
                self.canvas.create_text(
                    cx, cy - 14,
                    text=act.get("icon", "•"),
                    font=("Inter", 32, "bold"),
                    fill=self.colors["slice_active"]
                )

                # Action Title
                self.canvas.create_text(
                    cx, cy + 24,
                    text=act.get("label", ""),
                    font=("Inter", 13, "bold"),
                    fill=self.colors["text_primary"]
                )

                # Description or Shortcut Pill
                desc = act.get("desc", "")
                self.canvas.create_text(
                    cx, cy + 50,
                    text=desc,
                    font=("Inter", 8),
                    fill=self.colors["text_muted"]
                )

        elif self.current_mode in ("volume", "brightness"):
            mode_title = "Volume" if self.current_mode == "volume" else "Brightness"
            mode_icon = "🔊" if self.current_mode == "volume" else "☀️"

            self.canvas.create_text(
                cx, cy - 35,
                text=mode_icon,
                font=("Inter", 26, "bold"),
                fill=self.colors["gauge_fill"]
            )
            self.canvas.create_text(
                cx, cy + 5,
                text=f"{int(self.active_gauge_val)}%",
                font=("Inter", 28, "bold"),
                fill=self.colors["text_primary"]
            )
            self.canvas.create_text(
                cx, cy + 42,
                text=f"{mode_title} Mode",
                font=("Inter", 10),
                fill=self.colors["text_muted"]
            )

    def on_canvas_click(self, event):
        dx = event.x - self.center_x
        dy = event.y - self.center_y
        dist = math.sqrt(dx * dx + dy * dy)

        # Center Knob Click
        if dist <= self.r_inner - 10:
            self.on_click_event()
            return

        # Donut Sector Click
        if self.r_inner <= dist <= self.r_outer + 15 and self.current_mode == "menu":
            angle = math.atan2(dy, dx)
            # Map [-pi, pi] starting from 12 o'clock (-pi/2) to [0, 2*pi]
            norm_angle = (angle + (math.pi / 2.0)) % (2 * math.pi)
            n = len(self.actions)
            if n > 0:
                idx = int((norm_angle / (2 * math.pi)) * n) % n
                if idx == self.selected_index:
                    self.on_click_event()
                else:
                    self.selected_index = idx
                    self.draw_ui()


if __name__ == "__main__":
    app = DonutRadialHUD()
    app.show_hud()
    print("Donut Radial HUD preview running! Press Up/Down or Scroll to rotate, Enter to select, Esc to close.")
    app.mainloop()
