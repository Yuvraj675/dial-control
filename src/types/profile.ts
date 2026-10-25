export type ActionType =
  | "mode"
  | "hotkey"
  | "key"
  | "command"
  | "launch"
  | "text"
  | "macro"
  | "mute"
  | "home"
  | "profile"
  | "close";

/** One step of a macro slice; executed in order by the backend (`actions::Step`). */
export type MacroStep =
  | { kind: "hotkey"; keys: string[]; keys_mac?: string[] }
  | { kind: "text"; text: string }
  | { kind: "wait"; ms: number }
  | { kind: "launch"; target: string }
  | { kind: "command"; cmd: string }
  | { kind: "media"; action: "playpause" | "next" | "prev" };

export type MacroStepKind = MacroStep["kind"];

/** An application found on this computer, as returned by `list_installed_apps`. */
export interface InstalledApp {
  name: string;
  target: string;
}

export interface WheelAction {
  id: string;
  label: string;
  icon: string;
  type: ActionType;
  action?: string;       // For media keys: playpause, next, prev
  keys?: string[];       // For hotkeys: ["CTRL", "C"] (Windows & Linux)
  keys_mac?: string[];   // Same shortcut authored for macOS (Meta = Command); used as written
  cmd?: string;          // For shell commands
  target?: string;       // For launch: app id / shortcut / .app, executable, file, folder or URL
  text?: string;         // For typed text
  steps?: MacroStep[];   // For macros
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
