#!/usr/bin/env python3
"""
Cross-Platform Context Detector
Detects the active foreground application to provide context-aware dial profiles.
"""

import sys
import fnmatch
from typing import Tuple, Optional, Dict, Any

class ContextDetector:
    def __init__(self, profiles_config: Optional[Dict[str, Any]] = None):
        self.profiles_config = profiles_config or {}
        self.platform = sys.platform
        self._init_platform()

    def _init_platform(self):
        if self.platform.startswith("linux"):
            try:
                import gi
                gi.require_version('Atspi', '2.0')
                from gi.repository import Atspi
                self.Atspi = Atspi
                self.has_atspi = True
            except Exception:
                self.has_atspi = False
        else:
            self.has_atspi = False

    def get_active_window_info(self) -> Tuple[str, str]:
        """Returns (app_name, window_title)."""
        if self.platform.startswith("linux") and self.has_atspi:
            try:
                desktop = self.Atspi.get_desktop(0)
                for i in range(desktop.get_child_count()):
                    app = desktop.get_child_at_index(i)
                    states = app.get_state_set().get_states()
                    if self.Atspi.StateType.ACTIVE in states:
                        app_name = app.get_name()
                        title = ""
                        for j in range(app.get_child_count()):
                            win = app.get_child_at_index(j)
                            if self.Atspi.StateType.ACTIVE in win.get_state_set().get_states():
                                title = win.get_name()
                                break
                        return app_name, title
            except Exception:
                pass

        elif self.platform == "win32":
            try:
                import win32gui
                hwnd = win32gui.GetForegroundWindow()
                title = win32gui.GetWindowText(hwnd)
                return "windows_app", title
            except Exception:
                pass

        elif self.platform == "darwin":
            try:
                from AppKit import NSWorkspace
                app = NSWorkspace.sharedWorkspace().frontmostApplication()
                return app.localizedName(), ""
            except Exception:
                pass

        return "default", ""

    def match_profile(self, app_name: str, window_title: str) -> Tuple[str, Dict[str, Any]]:
        """Matches active app against configured profiles."""
        profiles = self.profiles_config.get("profiles", {})
        query = f"{app_name.lower()} {window_title.lower()}"

        for profile_id, data in profiles.items():
            if profile_id == "default":
                continue
            for pattern in data.get("match_patterns", []):
                if fnmatch.fnmatch(app_name.lower(), pattern.lower()) or fnmatch.fnmatch(query, pattern.lower()):
                    return profile_id, data

        default_profile = profiles.get("default", {"name": "System Controls", "actions": []})
        return "default", default_profile


if __name__ == "__main__":
    import json
    import os

    config_path = os.path.join(os.path.dirname(__file__), "..", "config", "profiles.json")
    with open(config_path, "r") as f:
        config = json.load(f)

    detector = ContextDetector(config)
    app, title = detector.get_active_window_info()
    p_id, profile = detector.match_profile(app, title)
    print(f"Active App: {app} | Title: {title}")
    print(f"Matched Profile: {p_id} ({profile.get('name')}) with {len(profile.get('actions', []))} actions.")
