#!/usr/bin/env python3
"""
System Tray Daemon for Dial Control
Runs quietly in the system tray (AppIndicator), provides quick profile switching,
settings launcher, and dial toggling, similar to Filee.
"""

import sys
import os
import subprocess
import signal

import gi
gi.require_version('Gtk', '3.0')
try:
    gi.require_version('AyatanaAppIndicator3', '0.1')
    from gi.repository import AyatanaAppIndicator3 as appindicator
except Exception:
    gi.require_version('AppIndicator3', '0.1')
    from gi.repository import AppIndicator3 as appindicator

from gi.repository import Gtk, GLib

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


class DialTrayApp:
    def __init__(self):
        self.indicator = appindicator.Indicator.new(
            "dial-control-tray",
            "audio-volume-high-symbolic",
            appindicator.IndicatorCategory.APPLICATION_STATUS
        )
        self.indicator.set_status(appindicator.IndicatorStatus.ACTIVE)
        self.build_menu()

    def build_menu(self):
        menu = Gtk.Menu()

        # Title
        item_title = Gtk.MenuItem(label="🎛️ CMF Smart Dial Active")
        item_title.set_sensitive(False)
        menu.append(item_title)

        menu.append(Gtk.SeparatorMenuItem())

        # Toggle Dial HUD
        item_toggle = Gtk.MenuItem(label="🔘 Show / Hide Radial Dial")
        item_toggle.connect("activate", self.on_toggle_dial)
        menu.append(item_toggle)

        # Settings
        item_settings = Gtk.MenuItem(label="⚙️ Settings & Customizer...")
        item_settings.connect("activate", self.on_open_settings)
        menu.append(item_settings)

        menu.append(Gtk.SeparatorMenuItem())

        # Quit
        item_quit = Gtk.MenuItem(label="❌ Quit Dial Daemon")
        item_quit.connect("activate", self.on_quit)
        menu.append(item_quit)

        menu.show_all()
        self.indicator.set_menu(menu)

    def on_toggle_dial(self, widget):
        # Trigger dial overlay
        hud_script = os.path.join(ROOT_DIR, "tools", "donut_dial.py")
        subprocess.Popen([sys.executable, hud_script])

    def on_open_settings(self, widget):
        settings_script = os.path.join(ROOT_DIR, "tools", "settings_gui.py")
        subprocess.Popen([sys.executable, settings_script])

    def on_quit(self, widget):
        Gtk.main_quit()

    def run(self):
        # Handle SIGINT cleanly
        signal.signal(signal.SIGINT, lambda s, f: Gtk.main_quit())
        signal.signal(signal.SIGTERM, lambda s, f: Gtk.main_quit())
        Gtk.main()


if __name__ == "__main__":
    print("[+] Starting Dial Control System Tray Daemon...")
    app = DialTrayApp()
    app.run()
