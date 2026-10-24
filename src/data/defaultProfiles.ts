import type { ProfilesConfig, WheelAction } from "../types/profile.ts";

export const DEFAULT_CONFIG: ProfilesConfig = {
  settings: {
    theme: "nothing_red",
    dial_radius: 185,
    donut_thickness: 64,
    sensitivity: 1.0,
    haptic_beep: true,
    start_in_tray: true,
    auto_close_seconds: 6,
    activation_shortcut: "Super+Shift+D",
  },
  profile_order: ["default", "media_video", "code_editor", "file_manager", "browser_nav", "creative_design"],
  profiles: {
    default: {
      id: "default",
      name: "System",
      icon: "sliders",
      match_patterns: ["*"],
      actions: [
        { id: "media_playpause", label: "PLAY / PAUSE", icon: "playpause", type: "key", action: "playpause" },
        { id: "brightness_mode", label: "BRIGHTNESS", icon: "brightness", type: "mode" },
        { id: "volume_mode", label: "VOLUME", icon: "volume", type: "mode" },
        { id: "scroll_mode", label: "SCROLL", icon: "scroll", type: "mode", action: "scroll", desc: "Rotate to scroll" },
        { id: "media_next", label: "NEXT TRACK", icon: "next", type: "key", action: "next" },
        { id: "media_prev", label: "PREV TRACK", icon: "prev", type: "key", action: "prev" },
        { id: "mute_toggle", label: "MUTE", icon: "mute", type: "mute" },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
    media_video: {
      id: "media_video",
      name: "Media",
      icon: "video",
      match_patterns: ["*youtube*", "*netflix*", "*spotify*", "*vlc*", "*mpv*"],
      actions: [
        { id: "media_playpause", label: "PLAY / PAUSE", icon: "playpause", type: "key", action: "playpause" },
        { id: "seek_fwd", label: "SEEK +10S", icon: "next", type: "hotkey", keys: ["KEY_RIGHT"] },
        { id: "seek_bwd", label: "SEEK -10S", icon: "prev", type: "hotkey", keys: ["KEY_LEFT"] },
        { id: "fullscreen", label: "FULLSCREEN", icon: "monitor", type: "hotkey", keys: ["KEY_F"] },
        { id: "video_mute", label: "MUTE", icon: "mute", type: "hotkey", keys: ["KEY_M"] },
        { id: "volume_mode", label: "VOLUME", icon: "volume", type: "mode" },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
    code_editor: {
      id: "code_editor",
      name: "Code",
      icon: "terminal",
      match_patterns: ["*code*", "*cursor*", "*zed*", "*sublime*", "*nvim*"],
      actions: [
        { id: "scroll_mode", label: "SCROLL", icon: "scroll", type: "mode", action: "scroll", desc: "Rotate to scroll" },
        { id: "code_save", label: "SAVE ALL", icon: "save", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_S"] },
        { id: "code_format", label: "FORMAT", icon: "format", type: "hotkey", keys: ["KEY_LEFTSHIFT", "KEY_LEFTALT", "KEY_F"] },
        { id: "code_find", label: "FIND IN FILES", icon: "find", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_F"] },
        { id: "code_comment", label: "COMMENT", icon: "comment", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_SLASH"] },
        { id: "code_terminal", label: "TERMINAL", icon: "terminal", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_GRAVE"] },
        { id: "code_git", label: "GIT DIFF", icon: "git", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_G"] },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
    file_manager: {
      id: "file_manager",
      name: "Files",
      icon: "folder",
      match_patterns: ["*nautilus*", "*dolphin*", "*thunar*", "*pcmanfm*", "*filee*", "*explorer*"],
      actions: [
        { id: "scroll_mode", label: "SCROLL", icon: "scroll", type: "mode", action: "scroll", desc: "Rotate to scroll" },
        { id: "file_copy", label: "COPY", icon: "copy", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_C"] },
        { id: "file_paste", label: "PASTE", icon: "paste", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_V"] },
        { id: "file_trash", label: "DELETE", icon: "trash", type: "hotkey", keys: ["KEY_DELETE"] },
        { id: "file_new_folder", label: "NEW FOLDER", icon: "new_folder", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_N"] },
        { id: "file_rename", label: "RENAME", icon: "rename", type: "hotkey", keys: ["KEY_F2"] },
        { id: "file_terminal", label: "TERMINAL", icon: "terminal", type: "command", cmd: "x-terminal-emulator || ptyxis || gnome-terminal || kgx || konsole || alacritty || kitty || xterm" },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
    browser_nav: {
      id: "browser_nav",
      name: "Browser",
      icon: "globe",
      match_patterns: ["*firefox*", "*chrome*", "*zen*", "*brave*", "*edge*"],
      actions: [
        { id: "scroll_mode", label: "SCROLL", icon: "scroll", type: "mode", action: "scroll", desc: "Rotate to scroll" },
        { id: "nav_back", label: "BACK", icon: "back", type: "hotkey", keys: ["KEY_LEFTALT", "KEY_LEFT"] },
        { id: "nav_fwd", label: "FORWARD", icon: "forward", type: "hotkey", keys: ["KEY_LEFTALT", "KEY_RIGHT"] },
        { id: "nav_reload", label: "RELOAD", icon: "reload", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_R"] },
        { id: "nav_new_tab", label: "NEW TAB", icon: "new_tab", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_T"] },
        { id: "nav_close_tab", label: "CLOSE TAB", icon: "close_tab", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_W"] },
        { id: "nav_reopen_tab", label: "RESTORE TAB", icon: "reopen_tab", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_T"] },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
    creative_design: {
      id: "creative_design",
      name: "Design",
      icon: "palette",
      match_patterns: ["*figma*", "*blender*", "*resolve*", "*photoshop*", "*inkscape*"],
      actions: [
        { id: "draw_undo", label: "UNDO", icon: "undo", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_Z"] },
        { id: "draw_redo", label: "REDO", icon: "redo", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_Y"] },
        { id: "draw_zoom_in", label: "ZOOM IN", icon: "zoom_in", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_EQUAL"] },
        { id: "draw_zoom_out", label: "ZOOM OUT", icon: "zoom_out", type: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_MINUS"] },
        { id: "draw_hand", label: "PAN", icon: "pan", type: "hotkey", keys: ["KEY_H"] },
        { id: "draw_select", label: "SELECT", icon: "select", type: "hotkey", keys: ["KEY_V"] },
        { id: "home_dial", label: "HOME", icon: "home", type: "home", desc: "Choose Dial Menu" },
        { id: "close_hud", label: "CLOSE", icon: "close", type: "close", desc: "Dismiss HUD" },
      ],
    },
  },
};

export const SYSTEM_ACTION_TEMPLATES: { action: WheelAction; sourceName: string }[] = [
  {
    action: {
      id: "scroll_mode",
      label: "SCROLL",
      icon: "scroll",
      type: "mode",
      action: "scroll",
      desc: "Rotate to scroll up or down",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "volume_mode",
      label: "VOLUME",
      icon: "volume",
      type: "mode",
      action: "volume",
      desc: "Rotate to adjust system volume",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "brightness_mode",
      label: "BRIGHTNESS",
      icon: "brightness",
      type: "mode",
      action: "brightness",
      desc: "Rotate to adjust display brightness",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "media_playpause",
      label: "PLAY / PAUSE",
      icon: "playpause",
      type: "key",
      action: "playpause",
      desc: "Toggle audio/video playback",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "media_next",
      label: "NEXT TRACK",
      icon: "next",
      type: "key",
      action: "next",
      desc: "Skip to next track",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "media_prev",
      label: "PREV TRACK",
      icon: "prev",
      type: "key",
      action: "prev",
      desc: "Return to previous track",
    },
    sourceName: "System Template",
  },
  {
    action: {
      id: "mute_toggle",
      label: "MUTE",
      icon: "mute",
      type: "mute",
      desc: "Toggle audio mute",
    },
    sourceName: "System Template",
  },
];

export function sanitizeAndMigrateConfig(cfg: ProfilesConfig): ProfilesConfig {
  if (!cfg || !cfg.profiles) return DEFAULT_CONFIG;

  const updatedProfiles = { ...cfg.profiles };

  // 1. Ensure 'default' (System) dial has Scroll Mode
  if (updatedProfiles.default) {
    const prof = { ...updatedProfiles.default };
    const hasScroll = prof.actions.some(
      (a) =>
        a.action === "scroll" ||
        a.id === "scroll_mode" ||
        a.id?.includes("scroll") ||
        a.label?.toUpperCase() === "SCROLL"
    );
    if (!hasScroll) {
      const scrollAction: WheelAction = {
        id: "scroll_mode",
        label: "SCROLL",
        icon: "scroll",
        type: "mode",
        action: "scroll",
        desc: "Rotate to scroll",
      };
      const volIdx = prof.actions.findIndex(
        (a) => a.id === "volume_mode" || a.action === "volume"
      );
      const insertIdx = volIdx !== -1 ? volIdx + 1 : 3;
      const newActions = [...prof.actions];
      newActions.splice(insertIdx, 0, scrollAction);
      prof.actions = newActions;
      updatedProfiles.default = prof;
    }
  }

  // 2. Ensure 'browser_nav' (Browser) dial has Scroll Mode
  if (updatedProfiles.browser_nav) {
    const prof = { ...updatedProfiles.browser_nav };
    const hasScroll = prof.actions.some(
      (a) =>
        a.action === "scroll" ||
        a.id === "scroll_mode" ||
        a.id?.includes("scroll") ||
        a.label?.toUpperCase() === "SCROLL"
    );
    if (!hasScroll) {
      const scrollAction: WheelAction = {
        id: "scroll_mode",
        label: "SCROLL",
        icon: "scroll",
        type: "mode",
        action: "scroll",
        desc: "Rotate to scroll",
      };
      prof.actions = [scrollAction, ...prof.actions];
      updatedProfiles.browser_nav = prof;
    }
  }

  // 3. Ensure 'code_editor' (Code) dial has Scroll Mode
  if (updatedProfiles.code_editor) {
    const prof = { ...updatedProfiles.code_editor };
    const hasScroll = prof.actions.some(
      (a) =>
        a.action === "scroll" ||
        a.id === "scroll_mode" ||
        a.id?.includes("scroll") ||
        a.label?.toUpperCase() === "SCROLL"
    );
    if (!hasScroll) {
      const scrollAction: WheelAction = {
        id: "scroll_mode",
        label: "SCROLL",
        icon: "scroll",
        type: "mode",
        action: "scroll",
        desc: "Rotate to scroll",
      };
      prof.actions = [scrollAction, ...prof.actions];
      updatedProfiles.code_editor = prof;
    }
  }

  // 4. Ensure 'file_manager' (Files) dial has Scroll Mode
  if (updatedProfiles.file_manager) {
    const prof = { ...updatedProfiles.file_manager };
    const hasScroll = prof.actions.some(
      (a) =>
        a.action === "scroll" ||
        a.id === "scroll_mode" ||
        a.id?.includes("scroll") ||
        a.label?.toUpperCase() === "SCROLL"
    );
    if (!hasScroll) {
      const scrollAction: WheelAction = {
        id: "scroll_mode",
        label: "SCROLL",
        icon: "scroll",
        type: "mode",
        action: "scroll",
        desc: "Rotate to scroll",
      };
      prof.actions = [scrollAction, ...prof.actions];
      updatedProfiles.file_manager = prof;
    }
  }

  // 5. Ensure every profile has HOME and CLOSE dial slices
  for (const prof of Object.values(updatedProfiles)) {
    if (!prof.actions.some((a) => a.id === "home_dial" || a.type === "home")) {
      prof.actions.push({
        id: "home_dial",
        label: "HOME",
        icon: "home",
        type: "home",
        desc: "Choose Dial Menu",
      });
    }
    if (!prof.actions.some((a) => a.id === "close_hud" || a.type === "close")) {
      prof.actions.push({
        id: "close_hud",
        label: "CLOSE",
        icon: "close",
        type: "close",
        desc: "Dismiss HUD",
      });
    }
  }

  return {
    ...cfg,
    profiles: updatedProfiles,
  };
}
