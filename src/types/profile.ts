export type ActionType = "mode" | "hotkey" | "key" | "command" | "mute" | "home" | "profile" | "close";

export interface WheelAction {
  id: string;
  label: string;
  icon: string;
  type: ActionType;
  action?: string;       // For media keys: playpause, next, prev
  keys?: string[];       // For hotkeys: ["CTRL", "C"]
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
}

export interface ProfilesConfig {
  settings: AppSettings;
  profiles: Record<string, DialProfile>;
}
