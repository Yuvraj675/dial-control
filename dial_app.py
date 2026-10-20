#!/usr/bin/env python3
"""
Dial Control - Main Unified Application
Integrates the Hardware Daemon, Context-Aware Donut HUD, Settings Customizer,
and System Tray into a single cohesive desktop application like Filee.
"""

import sys
import os
import signal
import threading
import subprocess
import time

# Ensure project root is in python path
ROOT_DIR = os.path.dirname(os.path.abspath(__file__))
if ROOT_DIR not in sys.path:
    sys.path.insert(0, ROOT_DIR)

from tools.avrcp_dial import DualPairingDialDaemon, find_cmf_avrcp_device
from tools.settings_gui import SettingsApp


def notify(title: str, msg: str):
    """Send desktop notification via notify-send."""
    try:
        icon_path = os.path.join(ROOT_DIR, "assets", "dial-control.png")
        subprocess.run(["notify-send", "-i", icon_path, title, msg], check=False)
    except Exception:
        pass


class UnifiedDialApp:
    def __init__(self):
        self.daemon = None
        self.settings_window = None
        self.tray_indicator = None

        # Lockfile check to prevent multiple instances
        self.lockfile = os.path.expanduser("~/.cache/dial-control.lock")
        os.makedirs(os.path.dirname(self.lockfile), exist_ok=True)
        if self._is_already_running():
            print("[*] Dial Control is already running. Opening settings window...")
            notify("Dial Control", "Application is already running in your system tray.")
            # Launch settings and exit
            subprocess.Popen([sys.executable, os.path.join(ROOT_DIR, "tools", "settings_gui.py")])
            sys.exit(0)

        self._create_lock()

    def _is_already_running(self) -> bool:
        if os.path.exists(self.lockfile):
            try:
                with open(self.lockfile, "r") as f:
                    pid = int(f.read().strip())
                # Check if process is still alive
                os.kill(pid, 0)
                return True
            except (ValueError, OSError):
                return False
        return False

    def _create_lock(self):
        try:
            with open(self.lockfile, "w") as f:
                f.write(str(os.getpid()))
        except Exception:
            pass

    def _remove_lock(self):
        try:
            if os.path.exists(self.lockfile):
                os.remove(self.lockfile)
        except Exception:
            pass

    def setup_tray(self):
        """Initialize system tray using AppIndicator in a dedicated GLib thread."""
        try:
            import gi
            gi.require_version('Gtk', '3.0')
            try:
                gi.require_version('AyatanaAppIndicator3', '0.1')
                from gi.repository import AyatanaAppIndicator3 as appindicator
            except Exception:
                gi.require_version('AppIndicator3', '0.1')
                from gi.repository import AppIndicator3 as appindicator
            from gi.repository import Gtk

            icon_path = os.path.join(ROOT_DIR, "assets", "dial-control.png")
            ind = appindicator.Indicator.new(
                "dial-control-tray",
                icon_path,
                appindicator.IndicatorCategory.APPLICATION_STATUS
            )
            ind.set_status(appindicator.IndicatorStatus.ACTIVE)

            menu = Gtk.Menu()

            # Status Title
            title_item = Gtk.MenuItem(label="🎛️ CMF Dial Active")
            title_item.set_sensitive(False)
            menu.append(title_item)

            menu.append(Gtk.SeparatorMenuItem())

            # Show Dial HUD
            item_hud = Gtk.MenuItem(label="🔘 Show Radial Donut HUD")
            item_hud.connect("activate", lambda w: self.toggle_hud())
            menu.append(item_hud)

            # Settings Window
            item_settings = Gtk.MenuItem(label="⚙️ Settings & Customizer...")
            item_settings.connect("activate", lambda w: self.open_settings())
            menu.append(item_settings)

            menu.append(Gtk.SeparatorMenuItem())

            # Quit
            item_quit = Gtk.MenuItem(label="❌ Quit Dial Control")
            item_quit.connect("activate", lambda w: self.quit_app())
            menu.append(item_quit)

            menu.show_all()
            ind.set_menu(menu)
            self.tray_indicator = ind

            # Run Gtk main loop for tray in background thread
            def gtk_loop():
                Gtk.main()

            tray_thread = threading.Thread(target=gtk_loop, daemon=True)
            tray_thread.start()
            print("[+] System Tray Indicator loaded successfully!")
        except Exception as e:
            print(f"[!] Warning: Could not initialize system tray indicator: {e}")

    def toggle_hud(self):
        if self.daemon and self.daemon.overlay:
            self.daemon.overlay.after(0, self.daemon.overlay.toggle)

    def open_settings(self):
        """Open or focus the Settings Customizer window."""
        def run_gui():
            settings_script = os.path.join(ROOT_DIR, "tools", "settings_gui.py")
            subprocess.Popen([sys.executable, settings_script])

        threading.Thread(target=run_gui, daemon=True).start()

    def quit_app(self):
        print("\n[*] Shutting down Dial Control...")
        self._remove_lock()
        if self.daemon:
            self.daemon.shutdown()
        os._exit(0)

    def run(self):
        # Setup system tray
        self.setup_tray()

        # Handle signals cleanly
        signal.signal(signal.SIGINT, lambda s, f: self.quit_app())
        signal.signal(signal.SIGTERM, lambda s, f: self.quit_app())

        # Notify user that Dial Control is active
        notify("Dial Control", "Running in background system tray. Click dial or tray icon to open.")

        print("==================================================")
        print("🎛️  Dial Control — Unified Desktop Application")
        print("Inspired by Filee: Donut HUD + System Tray + Context-Aware Profiles")
        print("==================================================")

        # Initialize and run the Dual Pairing Dial Daemon
        try:
            self.daemon = DualPairingDialDaemon()
            self.daemon.run()
        except KeyboardInterrupt:
            self.quit_app()
        except Exception as e:
            print(f"[!] Error in Dial Control daemon: {e}")
            self.quit_app()


if __name__ == "__main__":
    app = UnifiedDialApp()
    app.run()
