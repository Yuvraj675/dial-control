import type { ProfilesConfig } from "../types/profile.ts";
import { getOrderedProfileIds } from "../types/profile.ts";

export interface ContextPayload {
  profile_id: string;
  app?: string;
  title?: string;
  wm_class?: string;
}

function escapeRegExp(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * A `*pattern*` matches the app identity (WM_CLASS / exe / bundle id) as a substring,
 * but the window title only as a whole word, so "*code*" never fires on "Unicode table"
 * and "*edge*" never fires on "Knowledge base".
 */
export function patternMatches(pattern: string, identity: string, title: string): boolean {
  const clean = pattern.replace(/\*/g, "").toLowerCase().trim();
  if (!clean) return false;
  if (identity.toLowerCase().includes(clean)) return true;
  const word = new RegExp(`(^|[^\\p{L}\\p{N}])${escapeRegExp(clean)}($|[^\\p{L}\\p{N}])`, "u");
  return word.test(title.toLowerCase());
}

/**
 * Patterns the user added on top of a built-in profile's defaults. The built-in defaults are
 * already applied (with app-before-title priority) by the backend's `match_profile`, so
 * re-running them here would only override that decision with a cruder one.
 */
export function customPatterns(profileId: string, patterns: string[], defaults: ProfilesConfig): string[] {
  const builtin = defaults.profiles[profileId]?.match_patterns ?? [];
  return patterns.filter((p) => !builtin.includes(p));
}

/** Resolves which profile a foreground-window change should activate. */
export function resolveContextProfile(
  config: ProfilesConfig,
  payload: ContextPayload,
  defaults: ProfilesConfig
): string {
  const identity = `${payload.wm_class || ""} ${payload.app || ""}`;
  const title = payload.title || "";

  for (const id of getOrderedProfileIds(config)) {
    if (id === "default" || id === "home") continue;
    const prof = config.profiles[id];
    if (!prof) continue;
    if (customPatterns(id, prof.match_patterns || [], defaults).some((p) => patternMatches(p, identity, title))) {
      return id;
    }
  }

  if (payload.profile_id && config.profiles[payload.profile_id]) return payload.profile_id;
  return config.profiles.default ? "default" : getOrderedProfileIds(config)[0] ?? "default";
}

/** Moves a wheel selection by `dir` steps with true modular wrap-around. */
export function wrapIndex(prev: number, dir: number, count: number): number {
  if (count <= 0) return 0;
  return (((prev + dir) % count) + count) % count;
}
