#!/usr/bin/env python3
"""CMF Buds Pro 2 — Smart Dial Daemon & Virtual Overlay Controller.

Architecture:
- Clicks: Captured from /dev/input/event* exclusively via EVIOCGRAB
  (Prevents the OS / media players from pausing content when clicking the dial!)
- Rotations: Captured via BlueZ D-Bus MediaTransport1 (Volume delta)
- Volume Lock: While navigating the menu (or adjusting brightness),
  the volume is locked in hardware via D-Bus echo suppression so navigating
  DOES NOT change your actual audio volume, while the earbuds still beep in your ear!
- System Brightness: Synchronized directly with GNOME Mutter DisplayConfig
  AND sysfs so the on-screen dial and the GNOME system brightness slider stay 100% in sync!
- Modes:
  - Menu Mode: Rotate to browse options; Click center to enter selected mode.
  - Active Modes (Volume / Brightness): Rotate to adjust level live with on-screen HUD; Click to confirm & exit.
  - Quick Actions (Play/Pause, Next Track, Prev Track, Mute): Click to execute & dismiss.
"""

import math
import sys
import os
import glob
import struct
import fcntl
import threading
import time
import subprocess
import tkinter as tk
import dbus
from dbus.mainloop.glib import DBusGMainLoop
from gi.repository import GLib

# Linux Input Event Format & Exclusive Grab
EVENT_FORMAT = "llHHi"
EVENT_SIZE = struct.calcsize(EVENT_FORMAT)
EV_KEY = 0x01
KEY_PLAYPAUSE = 164
KEY_PLAYCD = 200    # CMF Center Click (Play)
KEY_PAUSECD = 201   # CMF Center Click (Pause)
KEY_VOICECOMMAND = 582
EVIOCGRAB = 0x40044590  # Linux ioctl to grab input device exclusively

MENU_ACTIONS = [
    ("🔊 Volume", "volume_mode"),
    ("☀️ Brightness", "brightness_mode"),
    ("⏯️ Play / Pause", "media_playpause"),
    ("⏭️ Next Track", "media_next"),
    ("⏮️ Prev Track", "media_prev"),
    ("🔇 Mute Audio", "mute_toggle"),
]

CMF_MAC_FORMATTED = "2C_BE_EE_31_11_77"
CMF_SINK_NAME = "bluez_output.2C_BE_EE_31_11_77.1"


# ── User Environment Helpers ──────────────────────────────────────────────────

def get_target_uid() -> str:
    """Return user UID even when running under sudo."""
    return os.environ.get("SUDO_UID", str(os.getuid()))


def get_user_pulse_env() -> dict:
    """Environment variables to connect to user's PipeWire/PulseAudio server."""
    env = os.environ.copy()
    uid = get_target_uid()
    env["PULSE_SERVER"] = f"unix:/run/user/{uid}/pulse/native"
    env["XDG_RUNTIME_DIR"] = f"/run/user/{uid}"
    return env


def get_user_dbus_bus() -> dbus.bus.BusConnection:
    """Connect to user's D-Bus session bus (works even under sudo)."""
    uid = get_target_uid()
    return dbus.bus.BusConnection(f"unix:path=/run/user/{uid}/bus")


# ── Synchronized Brightness Controls ──────────────────────────────────────────

def get_screen_brightness() -> int:
    """Read screen brightness percentage (0-100), keeping in sync with GNOME system slider."""
    # 1. Try GNOME Mutter DisplayConfig (System Slider)
    try:
        bus = get_user_dbus_bus()
        obj = bus.get_object("org.gnome.Mutter.DisplayConfig", "/org/gnome/Mutter/DisplayConfig")
        props_iface = dbus.Interface(obj, "org.freedesktop.DBus.Properties")
        bl = props_iface.Get("org.gnome.Mutter.DisplayConfig", "Backlight")
        if bl and len(bl) > 1 and len(bl[1]) > 0:
            m = bl[1][0]
            val = int(m["value"])
            min_v = int(m.get("min", 0))
            max_v = int(m.get("max", 100))
            if max_v > min_v:
                pct = int(((val - min_v) / (max_v - min_v)) * 100)
                return max(0, min(100, pct))
    except Exception:
        pass

    # 2. Try sysfs /sys/class/backlight
    try:
        paths = glob.glob("/sys/class/backlight/*/brightness")
        if paths:
            with open(paths[0], "r") as f:
                curr = int(f.read().strip())
            max_path = paths[0].replace("brightness", "max_brightness")
            with open(max_path, "r") as f:
                max_b = int(f.read().strip())
            return int((curr / max_b) * 100)
    except Exception:
        pass

    return 50


def set_screen_brightness(target_pct: int) -> int:
    """Set screen brightness percentage (0-100), updating both hardware and GNOME system slider."""
    target_pct = max(5, min(100, target_pct))

    # 1. Update GNOME Mutter DisplayConfig (moves the system brightness slider!)
    try:
        bus = get_user_dbus_bus()
        obj = bus.get_object("org.gnome.Mutter.DisplayConfig", "/org/gnome/Mutter/DisplayConfig")
        props_iface = dbus.Interface(obj, "org.freedesktop.DBus.Properties")
        iface = dbus.Interface(obj, "org.gnome.Mutter.DisplayConfig")
        bl = props_iface.Get("org.gnome.Mutter.DisplayConfig", "Backlight")
        if bl and len(bl) > 1 and len(bl[1]) > 0:
            serial = bl[0]
            m = bl[1][0]
            conn = str(m["connector"])
            min_v = int(m.get("min", 10))
            max_v = int(m.get("max", 210))
            target_val = int(min_v + (max_v - min_v) * (target_pct / 100.0))
            iface.SetBacklight(serial, conn, dbus.Int32(target_val))
    except Exception as e:
        print(f"[!] Mutter SetBacklight error: {e}")

    # 2. Update physical backlight via sysfs (ensures hardware changes smoothly)
    try:
        paths = glob.glob("/sys/class/backlight/*/brightness")
        if paths:
            b_path = paths[0]
            max_path = b_path.replace("brightness", "max_brightness")
            with open(max_path, "r") as f:
                max_b = int(f.read().strip())
            new_b = max(int(max_b * 0.05), min(max_b, int(max_b * (target_pct / 100.0))))
            with open(b_path, "w") as f:
                f.write(str(new_b))
    except Exception as e:
        print(f"[!] Direct sysfs brightness error: {e}")

    return target_pct


def adjust_screen_brightness(delta_percent: int) -> int:
    """Adjust brightness by a relative delta percentage."""
    curr = get_screen_brightness()
    target = max(5, min(100, curr + delta_percent))
    return set_screen_brightness(target)


# ── Native Media & Audio Controls ─────────────────────────────────────────────

def control_mpris(action: str) -> bool:
    """Control active media player (Spotify, Firefox, Chrome, Zen, etc.) via MPRIS D-Bus."""
    try:
        bus = get_user_dbus_bus()
        for name in bus.list_names():
            if name.startswith("org.mpris.MediaPlayer2"):
                player = bus.get_object(name, "/org/mpris/MediaPlayer2")
                iface = dbus.Interface(player, "org.mpris.MediaPlayer2.Player")
                if action == "playpause":
                    iface.PlayPause()
                    print(f"[MPRIS] Toggled Play/Pause on {name}")
                elif action == "next":
                    iface.Next()
                    print(f"[MPRIS] Next track on {name}")
                elif action == "prev":
                    iface.Previous()
                    print(f"[MPRIS] Prev track on {name}")
                return True
    except Exception as e:
        print(f"[!] MPRIS error: {e}")

    # Fallback to playerctl
    try:
        cmd_map = {"playpause": "play-pause", "next": "next", "prev": "previous"}
        subprocess.run(
            ["playerctl", cmd_map.get(action, action)],
            env=get_user_pulse_env(),
            check=False,
        )
        return True
    except Exception:
        pass
    return False


def get_current_sink_volume() -> int:
    """Get PipeWire/PulseAudio sink volume percentage."""
    try:
        out = subprocess.check_output(
            ["pactl", "get-sink-volume", "@DEFAULT_SINK@"],
            env=get_user_pulse_env(),
            text=True,
            stderr=subprocess.DEVNULL,
        )
        for part in out.split():
            if part.endswith("%"):
                return int(part[:-1])
    except Exception:
        pass
    return 50


def set_sink_volume(vol_pct: int):
    """Set sink volume percentage."""
    try:
        subprocess.run(
            ["pactl", "set-sink-volume", "@DEFAULT_SINK@", f"{vol_pct}%"],
            env=get_user_pulse_env(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
    except Exception as e:
        print(f"[!] Volume set error: {e}")


def toggle_sink_mute():
    """Toggle sink mute status."""
    try:
        subprocess.run(
            ["pactl", "set-sink-mute", "@DEFAULT_SINK@", "toggle"],
            env=get_user_pulse_env(),
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        print("[Audio] Toggled mute.")
    except Exception as e:
        print(f"[!] Mute toggle error: {e}")


def find_cmf_avrcp_device():
    """Locate the /dev/input/event* path for CMF Buds Pro 2 (AVRCP)."""
    try:
        with open("/proc/bus/input/devices", "r") as f:
            devices = f.read().split("\n\n")

        for dev in devices:
            if "CMF Buds Pro 2" in dev and "AVRCP" in dev:
                for line in dev.split("\n"):
                    if "Handlers=" in line:
                        for part in line.split():
                            if part.startswith("event"):
                                return f"/dev/input/{part}"

        for dev in devices:
            if "AVRCP" in dev:
                for line in dev.split("\n"):
                    if "Handlers=" in line:
                        for part in line.split():
                            if part.startswith("event"):
                                return f"/dev/input/{part}"
    except Exception as e:
        print(f"[!] Error reading input devices: {e}")
    return None


# ── Virtual Dial GUI ──────────────────────────────────────────────────────────

class VirtualDialOverlay(tk.Tk):
    def __init__(self, on_mode_change_cb=None):
        super().__init__()
        self.title("CMF Smart Dial")
        self.overrideredirect(True)
        self.attributes("-topmost", True)
        self.configure(bg="#121212")
        self.on_mode_change_cb = on_mode_change_cb

        self.size = 360
        self.radius = 135
        self.center_x = self.size // 2
        self.center_y = self.size // 2

        # Modes: "menu", "volume", "brightness"
        self.current_mode = "menu"
        self.selected_index = 0
        self.active_val = 50

        # Center on screen
        screen_w = self.winfo_screenwidth()
        screen_h = self.winfo_screenheight()
        pos_x = (screen_w - self.size) // 2
        pos_y = (screen_h - self.size) // 2
        self.geometry(f"{self.size}x{self.size}+{pos_x}+{pos_y}")

        self.canvas = tk.Canvas(
            self,
            width=self.size,
            height=self.size,
            bg="#181818",
            highlightthickness=2,
            highlightbackground="#ff4500",
        )
        self.canvas.pack(fill=tk.BOTH, expand=True)

        # Controls & Hotkeys
        self.bind("<Escape>", lambda e: self.hide_overlay())
        self.bind("<Button-3>", lambda e: self.hide_overlay())  # Right-click dismisses
        self.canvas.bind("<Button-1>", self.on_canvas_click)

        # Mouse wheel support for testing without dial
        self.canvas.bind("<Button-4>", lambda e: self.on_rotate_event(-1))  # Scroll up
        self.canvas.bind("<Button-5>", lambda e: self.on_rotate_event(1))   # Scroll down

        # Start hidden
        self.withdraw()
        self.is_visible = False

    def show_overlay(self):
        self.current_mode = "menu"
        self.is_visible = True
        self.draw_ui()
        self.deiconify()
        self.lift()
        self.after(20, self.focus_force)
        if self.on_mode_change_cb:
            self.on_mode_change_cb(self.current_mode)

    def hide_overlay(self):
        self.is_visible = False
        self.current_mode = "menu"
        self.withdraw()
        if self.on_mode_change_cb:
            self.on_mode_change_cb("closed")

    def toggle(self):
        if self.is_visible:
            self.hide_overlay()
        else:
            self.show_overlay()

    def on_rotate_event(self, direction: int):
        """Handle rotation (CW: direction > 0, CCW: direction < 0)."""
        if not self.is_visible:
            return

        if self.current_mode == "menu":
            if direction > 0:
                self.selected_index = (self.selected_index + 1) % len(MENU_ACTIONS)
            else:
                self.selected_index = (self.selected_index - 1) % len(MENU_ACTIONS)
            self.draw_ui()

        elif self.current_mode == "volume":
            step = 5 if direction > 0 else -5
            curr = get_current_sink_volume()
            new_v = max(0, min(100, curr + step))
            set_sink_volume(new_v)
            self.active_val = new_v
            self.draw_ui()

        elif self.current_mode == "brightness":
            step = 5 if direction > 0 else -5
            new_b = adjust_screen_brightness(step)
            self.active_val = new_b
            self.draw_ui()

    def on_click_event(self):
        """Handle center dial click."""
        if not self.is_visible:
            self.show_overlay()
            return

        if self.current_mode == "menu":
            _, action_key = MENU_ACTIONS[self.selected_index]
            self.handle_menu_select(action_key)
        else:
            # Inside sub-mode (Volume / Brightness) -> Click confirms and closes HUD
            self.hide_overlay()

    def handle_menu_select(self, action_key: str):
        if action_key == "volume_mode":
            self.current_mode = "volume"
            self.active_val = get_current_sink_volume()
            if self.on_mode_change_cb:
                self.on_mode_change_cb(self.current_mode)
            self.draw_ui()

        elif action_key == "brightness_mode":
            self.current_mode = "brightness"
            self.active_val = get_screen_brightness()
            if self.on_mode_change_cb:
                self.on_mode_change_cb(self.current_mode)
            self.draw_ui()

        elif action_key == "media_playpause":
            control_mpris("playpause")
            self.hide_overlay()

        elif action_key == "media_next":
            control_mpris("next")
            self.hide_overlay()

        elif action_key == "media_prev":
            control_mpris("prev")
            self.hide_overlay()

        elif action_key == "mute_toggle":
            toggle_sink_mute()
            self.hide_overlay()

    def on_canvas_click(self, event):
        # Top-right close button
        if event.x > self.size - 40 and event.y < 40:
            self.hide_overlay()
            return

        # Center knob click -> select / confirm
        dx = event.x - self.center_x
        dy = event.y - self.center_y
        if math.sqrt(dx * dx + dy * dy) <= 60:
            self.on_click_event()
            return

        # Click on peripheral option nodes
        if self.current_mode == "menu":
            n = len(MENU_ACTIONS)
            angle_step = 2 * math.pi / n
            for i in range(n):
                angle = i * angle_step - (math.pi / 2)
                nx = self.center_x + (self.radius - 28) * math.cos(angle)
                ny = self.center_y + (self.radius - 28) * math.sin(angle)
                if math.sqrt((event.x - nx) ** 2 + (event.y - ny) ** 2) <= 24:
                    self.selected_index = i
                    self.draw_ui()
                    self.on_click_event()
                    return

    def draw_ui(self):
        self.canvas.delete("all")

        # Top-right Close Button
        self.canvas.create_text(
            self.size - 20, 20,
            text="✕",
            fill="#888888",
            font=("Sans", 13, "bold"),
        )

        if self.current_mode == "menu":
            self.draw_menu_mode()
        else:
            self.draw_slider_mode()

    def draw_menu_mode(self):
        n = len(MENU_ACTIONS)
        angle_step = 2 * math.pi / n

        # Outer track ring
        self.canvas.create_oval(
            self.center_x - self.radius,
            self.center_y - self.radius,
            self.center_x + self.radius,
            self.center_y + self.radius,
            outline="#262626",
            width=5,
        )

        # Action nodes
        for i, (label, _) in enumerate(MENU_ACTIONS):
            angle = i * angle_step - (math.pi / 2)
            x = self.center_x + (self.radius - 28) * math.cos(angle)
            y = self.center_y + (self.radius - 28) * math.sin(angle)

            is_sel = (i == self.selected_index)
            color = "#ff4500" if is_sel else "#202020"
            r = 20 if is_sel else 14

            self.canvas.create_oval(
                x - r, y - r, x + r, y + r,
                fill=color,
                outline="#ffffff" if is_sel else "#383838",
                width=2 if is_sel else 1,
            )

        # Center knob
        label, _ = MENU_ACTIONS[self.selected_index]
        self.canvas.create_oval(
            self.center_x - 58, self.center_y - 58,
            self.center_x + 58, self.center_y + 58,
            fill="#1a1a1a",
            outline="#ff4500",
            width=3,
        )
        self.canvas.create_text(
            self.center_x, self.center_y - 8,
            text=label,
            fill="#ffffff",
            font=("Sans", 10, "bold"),
            justify=tk.CENTER,
        )
        self.canvas.create_text(
            self.center_x, self.center_y + 16,
            text="Click to Select\nEsc to Close",
            fill="#777777",
            font=("Sans", 7),
            justify=tk.CENTER,
        )

    def draw_slider_mode(self):
        """Draw interactive gauge for Volume or Brightness."""
        is_vol = (self.current_mode == "volume")
        title = "🔊 Volume Level" if is_vol else "☀️ Screen Brightness"
        val = self.active_val

        # Progress Arc
        extent_angle = -270 * (val / 100.0)

        # Background track arc
        self.canvas.create_arc(
            self.center_x - self.radius + 15, self.center_y - self.radius + 15,
            self.center_x + self.radius - 15, self.center_y + self.radius - 15,
            start=135, extent=-270,
            outline="#2a2a2a", width=12, style=tk.ARC,
        )
        # Active arc
        if extent_angle != 0:
            self.canvas.create_arc(
                self.center_x - self.radius + 15, self.center_y - self.radius + 15,
                self.center_x + self.radius - 15, self.center_y + self.radius - 15,
                start=135, extent=extent_angle,
                outline="#ff4500", width=12, style=tk.ARC,
            )

        # Center Display
        self.canvas.create_text(
            self.center_x, self.center_y - 25,
            text=title,
            fill="#aaaaaa",
            font=("Sans", 9),
        )
        self.canvas.create_text(
            self.center_x, self.center_y + 8,
            text=f"{val}%",
            fill="#ffffff",
            font=("Sans", 26, "bold"),
        )
        self.canvas.create_text(
            self.center_x, self.center_y + 40,
            text="Rotate to Adjust\nClick to Confirm",
            fill="#ff4500",
            font=("Sans", 8),
            justify=tk.CENTER,
        )


# ── Daemon Controller ─────────────────────────────────────────────────────────

class DualPairingDialDaemon:
    def __init__(self):
        self.dev_path = find_cmf_avrcp_device()
        self.overlay = VirtualDialOverlay(on_mode_change_cb=self.on_overlay_mode_change)

        self.last_volume = None
        self.locked_hardware_vol = None
        self.ignore_restore_vol = None
        self.transport_path = None
        self.last_click_time = 0

        # D-Bus connection for Volume (Rotation) monitoring on System Bus
        DBusGMainLoop(set_as_default=True)
        self.bus = dbus.SystemBus()
        self.bus.add_signal_receiver(
            self.on_dbus_property_changed,
            signal_name="PropertiesChanged",
            dbus_interface="org.freedesktop.DBus.Properties",
            path_keyword="path",
        )

        self.init_hardware_transport()

    def init_hardware_transport(self):
        """Find the CMF Buds Pro 2 MediaTransport1 object and read initial volume."""
        try:
            manager = dbus.Interface(
                self.bus.get_object("org.bluez", "/"),
                "org.freedesktop.DBus.ObjectManager",
            )
            objects = manager.GetManagedObjects()
            for path, ifaces in objects.items():
                if CMF_MAC_FORMATTED in path and "org.bluez.MediaTransport1" in ifaces:
                    self.transport_path = path
                    vol = ifaces["org.bluez.MediaTransport1"].get("Volume")
                    if vol is not None:
                        self.last_volume = int(vol)
                        print(f"[+] Initial Hardware Volume: {self.last_volume} (Transport: {path})")
                    break
        except Exception as e:
            print(f"[!] Could not locate BlueZ transport: {e}")

    def on_overlay_mode_change(self, new_mode: str):
        """Called when overlay switches mode or closes."""
        if new_mode == "closed":
            self.locked_hardware_vol = None
            self.ignore_restore_vol = None

    def on_dbus_property_changed(self, iface, changed, invalidated, path):
        """Called whenever the dial is rotated (Bluetooth Absolute Volume change)."""
        if iface != "org.bluez.MediaTransport1" or "Volume" not in changed:
            return

        new_vol = int(changed["Volume"])

        # Remember transport path if not known yet
        if not self.transport_path and CMF_MAC_FORMATTED in path:
            self.transport_path = path

        # 1. ECHO SUPPRESSION:
        # If this event matches our locked volume restoration, ignore it!
        if self.ignore_restore_vol is not None and abs(new_vol - self.ignore_restore_vol) <= 2:
            self.ignore_restore_vol = None
            self.last_volume = new_vol
            return

        if self.last_volume is None:
            self.last_volume = new_vol
            return

        delta = new_vol - self.last_volume
        self.last_volume = new_vol

        if delta == 0:
            return

        direction = 1 if delta > 0 else -1
        dir_name = "CW (+)" if delta > 0 else "CCW (-)"
        print(f">> [DIAL TURN] {dir_name} (Volume: {new_vol})")

        # Handle based on overlay state
        if self.overlay.is_visible:
            # 2. Advance menu or adjust sub-mode gauge
            self.overlay.after(0, self.overlay.on_rotate_event, direction)

            # 3. VOLUME LOCK: While browsing menu or adjusting brightness,
            #    restore hardware volume back to the locked value so audio volume DOES NOT CHANGE!
            if self.overlay.current_mode != "volume":
                if self.locked_hardware_vol is not None and self.transport_path:
                    try:
                        self.ignore_restore_vol = self.locked_hardware_vol
                        props = dbus.Interface(
                            self.bus.get_object("org.bluez", self.transport_path),
                            "org.freedesktop.DBus.Properties",
                        )
                        props.Set(
                            "org.bluez.MediaTransport1",
                            "Volume",
                            dbus.UInt16(self.locked_hardware_vol),
                        )
                    except Exception as e:
                        print(f"[!] Volume lock restore error: {e}")
            else:
                # In volume mode: user explicitly wants to adjust volume
                self.locked_hardware_vol = new_vol
        else:
            # Overlay is closed: normal volume behavior
            pass

    def on_click_received(self):
        """Called when center dial is clicked."""
        now = time.time()
        if now - self.last_click_time < 0.25:
            return
        self.last_click_time = now

        print("\033[92m>> [DIAL CLICK]\033[0m")

        # When opening overlay: lock the current hardware volume
        if not self.overlay.is_visible:
            # Re-read fresh transport volume
            if self.transport_path:
                try:
                    props = dbus.Interface(
                        self.bus.get_object("org.bluez", self.transport_path),
                        "org.freedesktop.DBus.Properties",
                    )
                    curr_v = int(props.Get("org.bluez.MediaTransport1", "Volume"))
                    # Clamp between 15 and 112 so user has headroom to rotate in both directions
                    self.locked_hardware_vol = max(15, min(112, curr_v))
                    self.last_volume = curr_v
                except Exception:
                    self.locked_hardware_vol = 50
            else:
                self.locked_hardware_vol = 50
            print(f"   -> Volume locked at {self.locked_hardware_vol}/127 while navigating.")

        self.overlay.after(0, self.overlay.on_click_event)

    def start_input_listener(self):
        """Read click events from /dev/input/event* exclusively."""
        while True:
            dev = find_cmf_avrcp_device()
            if not dev or not os.path.exists(dev):
                time.sleep(2.0)
                continue

            self.dev_path = dev
            print(f"[+] Reading clicks from {self.dev_path}")
            f = None
            try:
                f = open(self.dev_path, "rb")
                # Exclusively grab the device so KEY_PLAYCD/KEY_PAUSECD
                # are NOT passed to GNOME/system media players!
                try:
                    fcntl.ioctl(f.fileno(), EVIOCGRAB, 1)
                    print(f"[+] Device {self.dev_path} grabbed exclusively! (Play/Pause blocked from system)")
                except Exception as ge:
                    print(f"[!] Warning: Could not grab device exclusively: {ge}")

                while True:
                    data = f.read(EVENT_SIZE)
                    if not data:
                        break
                    _, _, ev_type, ev_code, ev_value = struct.unpack(EVENT_FORMAT, data)
                    if ev_type == EV_KEY and ev_value == 1:
                        if ev_code in (KEY_PLAYCD, KEY_PAUSECD, KEY_PLAYPAUSE, KEY_VOICECOMMAND):
                            self.on_click_received()
            except PermissionError:
                print("[-] Permission denied reading input event.")
                print("    Run with: sudo python3 tools/avrcp_dial.py")
                break
            except Exception as e:
                print(f"[-] Input listener error ({e}), reconnecting in 2s...")
                time.sleep(2.0)
            finally:
                if f:
                    try:
                        fcntl.ioctl(f.fileno(), EVIOCGRAB, 0)
                    except Exception:
                        pass
                    try:
                        f.close()
                    except Exception:
                        pass

    def run(self):
        print("==================================================")
        print("CMF Buds Pro 2 Dual-Pairing Dial Controller Active")
        print("Phone connection & Dual-Pairing are 100% PRESERVED!")
        print("• Click Dial   -> Opens Virtual Dial (System Play/Pause Blocked!)")
        print("• Rotate Dial  -> Rotates Options (Audio Volume Locked!)")
        print("• Click Again  -> Selects Mode (Volume / Brightness / Media)")
        print("• Esc / Right-Click -> Dismisses Dial")
        print("==================================================")

        # Background thread for click events
        input_thread = threading.Thread(target=self.start_input_listener, daemon=True)
        input_thread.start()

        # Background thread for D-Bus event loop
        def dbus_loop():
            loop = GLib.MainLoop()
            loop.run()

        dbus_thread = threading.Thread(target=dbus_loop, daemon=True)
        dbus_thread.start()

        # Main thread runs Tkinter GUI event loop cleanly
        self.overlay.mainloop()


if __name__ == "__main__":
    daemon = DualPairingDialDaemon()
    daemon.run()
