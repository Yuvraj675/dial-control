import type { ProfilesConfig, WheelAction } from "../types/profile.ts";
import { DEFAULT_CONFIG } from "./defaultProfiles.ts";

/** Phone actions a `system` slice can run (Android backend: `actions/Actions.kt`). */
export const ANDROID_SYSTEM_ACTIONS: { action: string; label: string; icon: string }[] = [
  { action: "back", label: "BACK", icon: "back" },
  { action: "home", label: "HOME SCREEN", icon: "home" },
  { action: "recents", label: "RECENTS", icon: "recents" },
  { action: "notifications", label: "NOTIFICATIONS", icon: "notifications" },
  { action: "quick_settings", label: "QUICK SETTINGS", icon: "quick_settings" },
  { action: "screenshot", label: "SCREENSHOT", icon: "screenshot" },
  { action: "lock", label: "LOCK SCREEN", icon: "lock" },
  { action: "power", label: "POWER MENU", icon: "power" },
  { action: "split_screen", label: "SPLIT SCREEN", icon: "split_screen" },
  { action: "swipe_up", label: "NEXT", icon: "swipe_up" },
  { action: "swipe_down", label: "PREVIOUS", icon: "swipe_down" },
  { action: "torch", label: "TORCH", icon: "torch" },
  { action: "dnd", label: "DO NOT DISTURB", icon: "dnd" },
];

const sys = (action: string, label?: string): WheelAction => {
  const def = ANDROID_SYSTEM_ACTIONS.find((a) => a.action === action)!;
  return { id: `sys_${action}`, label: label ?? def.label, icon: def.icon, type: "system", action };
};

const playPause: WheelAction = { id: "media_playpause", label: "PLAY / PAUSE", icon: "playpause", type: "key", action: "playpause" };
const next: WheelAction = { id: "media_next", label: "NEXT TRACK", icon: "next", type: "key", action: "next" };
const prev: WheelAction = { id: "media_prev", label: "PREV TRACK", icon: "prev", type: "key", action: "prev" };
const scroll: WheelAction = { id: "scroll_mode", label: "SCROLL", icon: "scroll", type: "mode", action: "scroll" };
const volume: WheelAction = { id: "volume_mode", label: "VOLUME", icon: "volume", type: "mode" };
const brightness: WheelAction = { id: "brightness_mode", label: "BRIGHTNESS", icon: "brightness", type: "mode" };
const mute: WheelAction = { id: "mute_toggle", label: "MUTE", icon: "mute", type: "mute" };
const home: WheelAction = { id: "home_dial", label: "HOME", icon: "home", type: "home" };
const close: WheelAction = { id: "close_hud", label: "CLOSE", icon: "close", type: "close" };

/**
 * Phone counterpart of DEFAULT_CONFIG: same settings and wheel, but slices that work on Android
 * (desktop hotkeys cannot be injected there). `match_patterns` match the foreground package name.
 */
export const ANDROID_DEFAULT_CONFIG: ProfilesConfig = {
  settings: { ...DEFAULT_CONFIG.settings },
  // Music before Video: "com.google.android.apps.youtube.music" also contains "youtube".
  profile_order: ["default", "music", "media_video", "feeds"],
  profiles: {
    default: {
      id: "default",
      name: "System",
      icon: "sliders",
      match_patterns: ["*"],
      actions: [playPause, brightness, volume, scroll, next, sys("torch"), sys("notifications"), sys("screenshot"), home, close],
    },
    media_video: {
      id: "media_video",
      name: "Video",
      icon: "video",
      match_patterns: ["*youtube*", "*netflix*", "*avod*", "*vlc*", "*videoplayer*", "*hotstar*", "*primevideo*"],
      actions: [playPause, scroll, sys("swipe_up", "NEXT SHORT"), volume, brightness, sys("back"), home, close],
    },
    feeds: {
      id: "feeds",
      name: "Feeds",
      icon: "layers",
      match_patterns: ["*instagram*", "*musically*", "*reddit*", "*twitter*", "*facebook*", "*linkedin*", "*threads*"],
      actions: [scroll, sys("swipe_up"), sys("swipe_down"), volume, sys("back"), home, close],
    },
    music: {
      id: "music",
      name: "Music",
      icon: "playpause",
      match_patterns: ["*spotify*", "*youtube.music*", "*apple.android.music*", "*jiobeats*", "*soundcloud*"],
      actions: [playPause, next, prev, volume, mute, home, close],
    },
  },
};
