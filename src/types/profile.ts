export type ActionType = "mode" | "hotkey" | "key" | "command" | "mute" | "home" | "profile" | "close";

export interface WheelAction {
  id: string;
  label: string;
  icon: string;
  type: ActionType;
  action?: string;       // For media keys: playpause, next, prev
  keys?: string[];       // For hotkeys: ["CTRL", "C"] (Windows & Linux)
  keys_mac?: string[];   // Same shortcut authored for macOS (Meta = Command); used as written
  cmd?: string;          // For shell commands
  desc?: string;
}

export interface DialProfile {
  id: string;
  name: string;
  icon: string;
  match_patterns: string[];
  actions: WheelAction[];
}

export interface AppSettings {
  theme: "nothing_red" | "stealth" | "cmf_orange";
  dial_radius: number;
  donut_thickness: number;
  sensitivity: number;
  haptic_beep: boolean;
  start_in_tray: boolean;
  auto_close_seconds: number;
  activation_shortcut?: string;
}

export interface ProfilesConfig {
  settings: AppSettings;
  profiles: Record<string, DialProfile>;
  profile_order?: string[];
}

export function getOrderedProfileIds(config: ProfilesConfig): string[] {
  const allIds = Object.keys(config.profiles);
  if (!config.profile_order || config.profile_order.length === 0) {
    return allIds;
  }
  const ordered = config.profile_order.filter((id) => allIds.includes(id));
  for (const id of allIds) {
    if (!ordered.includes(id)) {
      ordered.push(id);
    }
  }
  return ordered;
}
