import type { MacroStep, MacroStepKind } from "../types/profile.ts";

/** Converts a combo like "Ctrl+Shift+P" into evdev-style key tokens (KEY_LEFTCTRL, ...). */
export function comboToTokens(combo: string): string[] {
  return combo.split("+").map((p) => {
    const clean = p.trim().toUpperCase();
    if (clean === "CTRL" || clean === "CONTROL") return "KEY_LEFTCTRL";
    if (clean === "SHIFT") return "KEY_LEFTSHIFT";
    if (clean === "ALT") return "KEY_LEFTALT";
    if (clean === "SUPER" || clean === "META" || clean === "WIN") return "KEY_LEFTMETA";
    if (clean === "SPACE") return "KEY_SPACE";
    if (clean === "ENTER") return "KEY_ENTER";
    if (clean === "TAB") return "KEY_TAB";
    if (clean === "ESC" || clean === "ESCAPE") return "KEY_ESC";
    if (clean === "BACKSLASH") return "KEY_BACKSLASH";
    if (clean === "SLASH") return "KEY_SLASH";
    if (clean === "GRAVE") return "KEY_GRAVE";
    if (clean === "MINUS") return "KEY_MINUS";
    if (clean === "EQUAL") return "KEY_EQUAL";
    if (clean === "LEFT") return "KEY_LEFT";
    if (clean === "RIGHT") return "KEY_RIGHT";
    if (clean === "UP") return "KEY_UP";
    if (clean === "DOWN") return "KEY_DOWN";
    if (clean === "DELETE" || clean === "DEL") return "KEY_DELETE";
    if (clean === "INSERT" || clean === "INS") return "KEY_INSERT";
    if (clean === "HOME") return "KEY_HOME";
    if (clean === "END") return "KEY_END";
    if (clean === "PAGEUP" || clean === "PGUP") return "KEY_PAGEUP";
    if (clean === "PAGEDOWN" || clean === "PGDN") return "KEY_PAGEDOWN";
    if (/^F\d+$/.test(clean)) return `KEY_${clean}`;
    if (clean.length === 1 && clean >= "A" && clean <= "Z") return `KEY_${clean}`;
    if (clean.length === 1 && clean >= "0" && clean <= "9") return `KEY_${clean}`;
    if (clean.startsWith("KEY_")) return clean;
    return `KEY_${clean}`;
  });
}

/** Parses the free-text key field ("Ctrl+C", "KEY_LEFTALT, KEY_F4") into key tokens. */
export function parseKeysInput(val: string): string[] {
  return val
    .split(/[,+]/)
    .map((s) => s.trim())
    .filter(Boolean)
    .map((p) => {
      const clean = p.toUpperCase();
      if (clean.startsWith("KEY_")) return clean;
      return comboToTokens(clean)[0] || `KEY_${clean}`;
    });
}

/** Short label for a key token: KEY_LEFTCTRL -> CTRL. */
export function formatTokenForDisplay(token: string): string {
  const clean = token.replace(/^KEY_/, "").toUpperCase();
  if (clean === "LEFTCTRL" || clean === "RIGHTCTRL") return "CTRL";
  if (clean === "LEFTSHIFT" || clean === "RIGHTSHIFT") return "SHIFT";
  if (clean === "LEFTALT" || clean === "RIGHTALT") return "ALT";
  if (clean === "LEFTMETA" || clean === "RIGHTMETA") return "SUPER";
  return clean;
}

/** `label` is the add-step button; `short` heads the step in the list. */
export const MACRO_STEP_KINDS: { kind: MacroStepKind; label: string; short: string }[] = [
  { kind: "hotkey", label: "HOTKEY", short: "HOTKEY" },
  { kind: "text", label: "TYPE TEXT", short: "TYPE" },
  { kind: "wait", label: "WAIT", short: "WAIT" },
  { kind: "launch", label: "OPEN APP / FILE / URL", short: "OPEN" },
  { kind: "command", label: "SHELL COMMAND", short: "COMMAND" },
  { kind: "media", label: "MEDIA KEY", short: "MEDIA" },
];

/** Longest wait the backend honours (actions::MAX_WAIT_MS). */
export const MAX_WAIT_MS = 10_000;

export function newStep(kind: MacroStepKind): MacroStep {
  switch (kind) {
    case "hotkey":
      return { kind, keys: [] };
    case "text":
      return { kind, text: "" };
    case "wait":
      return { kind, ms: 500 };
    case "launch":
      return { kind, target: "" };
    case "command":
      return { kind, cmd: "" };
    case "media":
      return { kind, action: "playpause" };
  }
}

/** Returns a copy of `items` with the element at `from` moved by `delta` (clamped). */
export function moveItem<T>(items: T[], from: number, delta: number): T[] {
  const to = Math.max(0, Math.min(items.length - 1, from + delta));
  if (from < 0 || from >= items.length || to === from) return items;
  const next = [...items];
  const [item] = next.splice(from, 1);
  next.splice(to, 0, item);
  return next;
}

/** One-line summary of a step, shown in the step list. */
export function describeStep(step: MacroStep): string {
  switch (step.kind) {
    case "hotkey":
      return step.keys.length ? step.keys.map(formatTokenForDisplay).join(" + ") : "NO KEYS SET";
    case "text":
      return step.text ? `"${step.text.length > 28 ? step.text.slice(0, 28) + "…" : step.text}"` : "NO TEXT";
    case "wait":
      return `${step.ms} MS`;
    case "launch":
      return step.target ? (step.target.split(/[\\/]/).pop() ?? step.target) : "NOTHING CHOSEN";
    case "command":
      return step.cmd || "NO COMMAND";
    case "media":
      return step.action === "next" ? "NEXT TRACK" : step.action === "prev" ? "PREVIOUS TRACK" : "PLAY / PAUSE";
  }
}
