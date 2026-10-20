#!/usr/bin/env python3
"""
Dial Control - Settings & Wheel Customizer GUI
Allows users to visually customize dial slices, app profiles, hotkeys, and appearance.
"""

import sys
import os
import json
import tkinter as tk
from tkinter import ttk, messagebox
from typing import Dict, Any, List

CONFIG_FILE = os.path.join(os.path.dirname(os.path.dirname(os.path.abspath(__file__))), "config", "profiles.json")


class SettingsApp(tk.Tk):
    def __init__(self):
        super().__init__()

        self.title("Dial Control - Settings & Wheel Customizer")
        self.geometry("740x560")
        self.minsize(680, 500)
        self.configure(bg="#121319")

        # Styling
        self.style = ttk.Style()
        self.style.theme_use("clam")
        self.setup_dark_theme()

        self.load_config()
        self.active_profile_key = "default"
        self.selected_action_idx = 0

        self.build_ui()
        self.populate_profiles()

    def setup_dark_theme(self):
        bg = "#121319"
        card_bg = "#1b1d26"
        fg = "#f0f2f8"
        accent = "#00d2ff"

        self.style.configure(".", background=bg, foreground=fg, font=("Inter", 10))
        self.style.configure("TFrame", background=bg)
        self.style.configure("Card.TFrame", background=card_bg, relief="flat")
        self.style.configure("TLabel", background=bg, foreground=fg)
        self.style.configure("Card.TLabel", background=card_bg, foreground=fg)
        self.style.configure("Title.TLabel", font=("Inter", 14, "bold"), foreground="#ffffff")
        self.style.configure("Subtitle.TLabel", font=("Inter", 9), foreground="#888c9e")

        self.style.configure("TButton", background="#2a2d3d", foreground="#ffffff", borderwidth=0, padding=6)
        self.style.map("TButton", background=[("active", "#363a4d"), ("pressed", "#00d2ff")])

        self.style.configure("Accent.TButton", background=accent, foreground="#000000", font=("Inter", 10, "bold"))
        self.style.map("Accent.TButton", background=[("active", "#33dcff")])

        self.style.configure("TCombobox", fieldbackground="#242735", background="#2a2d3d", foreground="#ffffff")

    def load_config(self):
        if os.path.exists(CONFIG_FILE):
            try:
                with open(CONFIG_FILE, "r", encoding="utf-8") as f:
                    self.config = json.load(f)
            except Exception:
                self.config = {"settings": {}, "profiles": {}}
        else:
            self.config = {"settings": {}, "profiles": {}}

    def save_config(self):
        os.makedirs(os.path.dirname(CONFIG_FILE), exist_ok=True)
        try:
            with open(CONFIG_FILE, "w", encoding="utf-8") as f:
                json.dump(self.config, f, indent=2)
            messagebox.showinfo("Saved", "Settings saved successfully! Active dial reloads automatically.")
        except Exception as e:
            messagebox.showerror("Error", f"Failed to save settings: {e}")

    def build_ui(self):
        # Header Bar
        header = ttk.Frame(self, padding=(20, 16, 20, 10))
        header.pack(fill="x")

        ttk.Label(header, text="🎛️ Dial Control Customizer", style="Title.TLabel").pack(anchor="w")
        ttk.Label(
            header,
            text="Customize context-aware wheel slices, per-app profiles, and hardware behavior",
            style="Subtitle.TLabel"
        ).pack(anchor="w")

        # Main Body Notebook (Tabs)
        self.notebook = ttk.Notebook(self)
        self.notebook.pack(fill="both", expand=True, padx=20, pady=10)

        # Tab 1: Wheel & Profiles
        self.tab_wheel = ttk.Frame(self.notebook, padding=15)
        self.notebook.add(self.tab_wheel, text="  🎡 Wheel Slices & Profiles  ")
        self.build_wheel_tab()

        # Tab 2: General Settings
        self.tab_general = ttk.Frame(self.notebook, padding=20)
        self.notebook.add(self.tab_general, text="  ⚙️ Preferences  ")
        self.build_general_tab()

        # Footer Bar
        footer = ttk.Frame(self, padding=(20, 10, 20, 15))
        footer.pack(fill="x")

        ttk.Button(footer, text="Save & Apply", style="Accent.TButton", command=self.save_config).pack(side="right", padx=5)
        ttk.Button(footer, text="Close", command=self.destroy).pack(side="right", padx=5)

    def build_wheel_tab(self):
        # Top Bar: Profile Selector
        top_bar = ttk.Frame(self.tab_wheel)
        top_bar.pack(fill="x", pady=(0, 12))

        ttk.Label(top_bar, text="Active Profile:").pack(side="left", padx=(0, 8))
        self.profile_var = tk.StringVar()
        self.profile_combo = ttk.Combobox(top_bar, textvariable=self.profile_var, state="readonly", width=25)
        self.profile_combo.pack(side="left", padx=(0, 12))
        self.profile_combo.bind("<<ComboboxSelected>>", self.on_profile_changed)

        ttk.Button(top_bar, text="+ New Profile", command=self.create_new_profile).pack(side="left", padx=4)

        # Split View: Left List of Actions, Right Action Editor
        content = ttk.Frame(self.tab_wheel)
        content.pack(fill="both", expand=True)

        # Left Column: Actions List
        left_card = ttk.Frame(content, style="Card.TFrame", padding=12)
        left_card.pack(side="left", fill="both", expand=False, padx=(0, 10))

        ttk.Label(left_card, text="Wheel Slices (Order on Dial)", font=("Inter", 10, "bold"), style="Card.TLabel").pack(anchor="w", pady=(0, 8))

        self.actions_listbox = tk.Listbox(
            left_card,
            bg="#232533",
            fg="#ffffff",
            selectbackground="#00d2ff",
            selectforeground="#000000",
            bd=0,
            highlightthickness=0,
            font=("Inter", 10),
            width=26,
            height=12
        )
        self.actions_listbox.pack(fill="both", expand=True, pady=(0, 8))
        self.actions_listbox.bind("<<ListboxSelect>>", self.on_action_selected)

        # Action reorder buttons
        btn_bar = ttk.Frame(left_card, style="Card.TFrame")
        btn_bar.pack(fill="x")
        ttk.Button(btn_bar, text="▲ Up", width=6, command=lambda: self.move_action(-1)).pack(side="left", padx=2)
        ttk.Button(btn_bar, text="▼ Down", width=6, command=lambda: self.move_action(1)).pack(side="left", padx=2)
        ttk.Button(btn_bar, text="+ Add", width=6, command=self.add_slice).pack(side="left", padx=2)
        ttk.Button(btn_bar, text="✕ Del", width=6, command=self.remove_slice).pack(side="left", padx=2)

        # Right Column: Slice Editor Card
        right_card = ttk.Frame(content, style="Card.TFrame", padding=16)
        right_card.pack(side="right", fill="both", expand=True)

        ttk.Label(right_card, text="Slice Details", font=("Inter", 11, "bold"), style="Card.TLabel").pack(anchor="w", pady=(0, 12))

        # Form Fields
        f_grid = ttk.Frame(right_card, style="Card.TFrame")
        f_grid.pack(fill="x", pady=4)

        # Label
        ttk.Label(f_grid, text="Action Label:", style="Card.TLabel").grid(row=0, column=0, sticky="w", pady=6)
        self.entry_label = tk.Entry(f_grid, bg="#242735", fg="#ffffff", insertbackground="#fff", bd=1, relief="solid")
        self.entry_label.grid(row=0, column=1, sticky="ew", padx=(10, 0), pady=6)

        # Icon
        ttk.Label(f_grid, text="Icon (Emoji/Glyph):", style="Card.TLabel").grid(row=1, column=0, sticky="w", pady=6)
        self.entry_icon = tk.Entry(f_grid, bg="#242735", fg="#ffffff", insertbackground="#fff", bd=1, relief="solid", width=8)
        self.entry_icon.grid(row=1, column=1, sticky="w", padx=(10, 0), pady=6)

        # Type
        ttk.Label(f_grid, text="Action Type:", style="Card.TLabel").grid(row=2, column=0, sticky="w", pady=6)
        self.type_var = tk.StringVar()
        self.type_combo = ttk.Combobox(f_grid, textvariable=self.type_var, state="readonly", values=["mode", "hotkey", "key", "command", "mute"])
        self.type_combo.grid(row=2, column=1, sticky="w", padx=(10, 0), pady=6)
        self.type_combo.bind("<<ComboboxSelected>>", self.on_type_changed)

        # Target (Hotkey keys or Command or Media action)
        ttk.Label(f_grid, text="Command / Key:", style="Card.TLabel").grid(row=3, column=0, sticky="w", pady=6)
        self.entry_target = tk.Entry(f_grid, bg="#242735", fg="#ffffff", insertbackground="#fff", bd=1, relief="solid")
        self.entry_target.grid(row=3, column=1, sticky="ew", padx=(10, 0), pady=6)

        # Description
        ttk.Label(f_grid, text="Description:", style="Card.TLabel").grid(row=4, column=0, sticky="w", pady=6)
        self.entry_desc = tk.Entry(f_grid, bg="#242735", fg="#ffffff", insertbackground="#fff", bd=1, relief="solid")
        self.entry_desc.grid(row=4, column=1, sticky="ew", padx=(10, 0), pady=6)

        f_grid.columnconfigure(1, weight=1)

        ttk.Button(right_card, text="Update Slice", style="Accent.TButton", command=self.apply_slice_edits).pack(anchor="e", pady=(15, 0))

    def build_general_tab(self):
        card = ttk.Frame(self.tab_general, style="Card.TFrame", padding=20)
        card.pack(fill="both", expand=True)

        settings = self.config.get("settings", {})

        # Dial Radius
        ttk.Label(card, text="Dial Outer Radius (pixels):", style="Card.TLabel").pack(anchor="w", pady=(0, 4))
        self.scale_radius = tk.Scale(card, from_=140, to_=240, orient="horizontal", bg="#1b1d26", fg="#fff", highlightthickness=0)
        self.scale_radius.set(settings.get("dial_radius", 180))
        self.scale_radius.pack(fill="x", pady=(0, 16))

        # Donut Thickness
        ttk.Label(card, text="Donut Ring Thickness (pixels):", style="Card.TLabel").pack(anchor="w", pady=(0, 4))
        self.scale_thick = tk.Scale(card, from_=40, to_=80, orient="horizontal", bg="#1b1d26", fg="#fff", highlightthickness=0)
        self.scale_thick.set(settings.get("donut_thickness", 60))
        self.scale_thick.pack(fill="x", pady=(0, 16))

        # Start in tray
        self.tray_var = tk.BooleanVar(value=settings.get("start_in_tray", True))
        chk_tray = tk.Checkbutton(
            card,
            text="Start in background system tray (like Filee)",
            variable=self.tray_var,
            bg="#1b1d26",
            fg="#fff",
            selectcolor="#121319",
            activebackground="#1b1d26",
            activeforeground="#fff"
        )
        chk_tray.pack(anchor="w", pady=8)

        # Haptic beep
        self.beep_var = tk.BooleanVar(value=settings.get("haptic_beep", True))
        chk_beep = tk.Checkbutton(
            card,
            text="Play hardware audio feedback beeps in earbuds on rotation",
            variable=self.beep_var,
            bg="#1b1d26",
            fg="#fff",
            selectcolor="#121319",
            activebackground="#1b1d26",
            activeforeground="#fff"
        )
        chk_beep.pack(anchor="w", pady=8)

    def populate_profiles(self):
        profiles = self.config.get("profiles", {})
        names = []
        self.profile_keys = []
        for k, v in profiles.items():
            self.profile_keys.append(k)
            names.append(f"{v.get('icon', '•')} {v.get('name', k)}")
        self.profile_combo["values"] = names
        if names:
            self.profile_combo.current(0)
            self.on_profile_changed()

    def on_profile_changed(self, event=None):
        idx = self.profile_combo.current()
        if 0 <= idx < len(self.profile_keys):
            self.active_profile_key = self.profile_keys[idx]
            self.refresh_actions_list()

    def refresh_actions_list(self):
        self.actions_listbox.delete(0, tk.END)
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.get("actions", [])
        for a in actions:
            self.actions_listbox.insert(tk.END, f"{a.get('icon', '•')}  {a.get('label', 'Action')}")
        if actions:
            self.actions_listbox.select_set(0)
            self.on_action_selected()

    def on_action_selected(self, event=None):
        sel = self.actions_listbox.curselection()
        if not sel:
            return
        idx = sel[0]
        self.selected_action_idx = idx
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.get("actions", [])
        if 0 <= idx < len(actions):
            act = actions[idx]
            self.entry_label.delete(0, tk.END)
            self.entry_label.insert(0, act.get("label", ""))

            self.entry_icon.delete(0, tk.END)
            self.entry_icon.insert(0, act.get("icon", ""))

            a_type = act.get("type", "hotkey")
            self.type_combo.set(a_type)

            target = ""
            if a_type == "hotkey":
                target = " + ".join(act.get("keys", []))
            elif a_type == "command":
                target = act.get("cmd", "")
            elif a_type == "key":
                target = act.get("action", "")
            elif a_type == "mode":
                target = act.get("id", "")

            self.entry_target.delete(0, tk.END)
            self.entry_target.insert(0, target)

            self.entry_desc.delete(0, tk.END)
            self.entry_desc.insert(0, act.get("desc", ""))

    def on_type_changed(self, event=None):
        pass

    def apply_slice_edits(self):
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.get("actions", [])
        if 0 <= self.selected_action_idx < len(actions):
            act = actions[self.selected_action_idx]
            act["label"] = self.entry_label.get().strip()
            act["icon"] = self.entry_icon.get().strip()
            a_type = self.type_combo.get().strip()
            act["type"] = a_type
            act["desc"] = self.entry_desc.get().strip()

            target = self.entry_target.get().strip()
            if a_type == "hotkey":
                # Parse e.g. "KEY_LEFTCTRL + KEY_C" or "Ctrl+C"
                parts = [p.strip().upper() for p in target.replace("+", " ").split()]
                keys = []
                for p in parts:
                    if not p.startswith("KEY_"):
                        if p in ("CTRL", "CONTROL"):
                            p = "KEY_LEFTCTRL"
                        elif p in ("SHIFT",):
                            p = "KEY_LEFTSHIFT"
                        elif p in ("ALT",):
                            p = "KEY_LEFTALT"
                        else:
                            p = f"KEY_{p}"
                    keys.append(p)
                act["keys"] = keys
            elif a_type == "command":
                act["cmd"] = target
            elif a_type == "key":
                act["action"] = target
            elif a_type == "mode":
                act["id"] = target

            self.refresh_actions_list()
            self.actions_listbox.select_set(self.selected_action_idx)

    def move_action(self, delta: int):
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.get("actions", [])
        idx = self.selected_action_idx
        new_idx = idx + delta
        if 0 <= new_idx < len(actions):
            actions[idx], actions[new_idx] = actions[new_idx], actions[idx]
            self.selected_action_idx = new_idx
            self.refresh_actions_list()
            self.actions_listbox.select_set(new_idx)

    def add_slice(self):
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.setdefault("actions", [])
        new_act = {
            "id": f"action_{len(actions)+1}",
            "label": "New Action",
            "icon": "⭐",
            "type": "hotkey",
            "keys": ["KEY_LEFTCTRL", "KEY_S"],
            "desc": "Custom shortcut"
        }
        actions.append(new_act)
        self.selected_action_idx = len(actions) - 1
        self.refresh_actions_list()
        self.actions_listbox.select_set(self.selected_action_idx)

    def remove_slice(self):
        profile = self.config.get("profiles", {}).get(self.active_profile_key, {})
        actions = profile.get("actions", [])
        if 0 <= self.selected_action_idx < len(actions) and len(actions) > 2:
            del actions[self.selected_action_idx]
            self.selected_action_idx = max(0, self.selected_action_idx - 1)
            self.refresh_actions_list()

    def create_new_profile(self):
        key = f"profile_{len(self.config.get('profiles', {}))+1}"
        self.config.setdefault("profiles", {})[key] = {
            "name": "Custom Profile",
            "icon": "✨",
            "match_patterns": ["*app*"],
            "actions": [
                {"id": "act1", "label": "Action 1", "icon": "1️⃣", "type": "hotkey", "keys": ["KEY_A"], "desc": "Action 1"},
                {"id": "act2", "label": "Action 2", "icon": "2️⃣", "type": "hotkey", "keys": ["KEY_B"], "desc": "Action 2"},
                {"id": "act3", "label": "Action 3", "icon": "3️⃣", "type": "hotkey", "keys": ["KEY_C"], "desc": "Action 3"}
            ]
        }
        self.populate_profiles()
        self.profile_combo.current(len(self.profile_keys) - 1)
        self.on_profile_changed()


if __name__ == "__main__":
    app = SettingsApp()
    app.mainloop()
