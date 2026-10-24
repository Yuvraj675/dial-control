import { test } from "node:test";
import assert from "node:assert/strict";
import { customPatterns, patternMatches, resolveContextProfile, wrapIndex } from "./contextMatch.ts";
import { DEFAULT_CONFIG } from "../data/defaultProfiles.ts";
import type { ProfilesConfig } from "../types/profile.ts";

function withCustomProfile(patterns: string[], extraOnCode: string[] = []): ProfilesConfig {
  const cfg: ProfilesConfig = structuredClone(DEFAULT_CONFIG);
  cfg.profiles.code_editor.match_patterns = [...cfg.profiles.code_editor.match_patterns, ...extraOnCode];
  cfg.profiles.notes = {
    id: "notes",
    name: "Notes",
    icon: "sliders",
    match_patterns: patterns,
    actions: [{ id: "close_hud", label: "CLOSE", icon: "close", type: "close" }],
  };
  cfg.profile_order = [...(cfg.profile_order ?? []), "notes"];
  return cfg;
}

test("identity matches as substring, title only as a whole word", () => {
  assert.equal(patternMatches("*code*", "com.microsoft.vscode", ""), true);
  assert.equal(patternMatches("*code*", "gnome-text-editor", "Unicode table"), false);
  assert.equal(patternMatches("*edge*", "libreoffice", "Knowledge base"), false);
  assert.equal(patternMatches("*zen*", "nautilus", "Citizen Kane"), false);
  assert.equal(patternMatches("*youtube*", "firefox", "Lofi - YouTube — Mozilla Firefox"), true);
  assert.equal(patternMatches("*obsidian*", "", "vault - Obsidian v1.6"), true);
});

test("empty and wildcard-only patterns never match", () => {
  assert.equal(patternMatches("*", "anything", "anything"), false);
  assert.equal(patternMatches("  ", "anything", "anything"), false);
  assert.equal(patternMatches("**", "", ""), false);
});

test("regex metacharacters in user patterns are literal", () => {
  assert.equal(patternMatches("*c++*", "", "main.cpp - c++ project"), true);
  assert.equal(patternMatches("*a.b*", "", "axb"), false);
  assert.doesNotThrow(() => patternMatches("*(unclosed*", "", "x"));
});

test("built-in default patterns are left to the backend", () => {
  assert.deepEqual(customPatterns("code_editor", DEFAULT_CONFIG.profiles.code_editor.match_patterns, DEFAULT_CONFIG), []);
  assert.deepEqual(customPatterns("code_editor", ["*code*", "*obsidian*"], DEFAULT_CONFIG), ["*obsidian*"]);
  assert.deepEqual(customPatterns("my_custom", ["*x*"], DEFAULT_CONFIG), ["*x*"]);
});

test("backend profile wins when no custom pattern applies", () => {
  // Regression: the frontend used to substring-match "*code*" against the title and
  // override the backend for a Chrome tab titled "Unicode converter".
  const id = resolveContextProfile(
    DEFAULT_CONFIG,
    { profile_id: "browser_nav", app: "chrome", wm_class: "google-chrome", title: "Unicode converter" },
    DEFAULT_CONFIG
  );
  assert.equal(id, "browser_nav");
});

test("custom profile patterns override the backend", () => {
  const cfg = withCustomProfile(["*obsidian*"]);
  const id = resolveContextProfile(cfg, { profile_id: "default", app: "obsidian", title: "Vault" }, DEFAULT_CONFIG);
  assert.equal(id, "notes");
});

test("patterns a user added to a built-in profile still apply", () => {
  const cfg = withCustomProfile([], ["*helix*"]);
  const id = resolveContextProfile(cfg, { profile_id: "default", app: "kitty", title: "helix main.rs" }, DEFAULT_CONFIG);
  assert.equal(id, "code_editor");
});

test("unknown backend profile ids fall back to default", () => {
  // Regression: macOS used to emit ids like "developer" that no profile has.
  const id = resolveContextProfile(DEFAULT_CONFIG, { profile_id: "developer", app: "Xcode" }, DEFAULT_CONFIG);
  assert.equal(id, "default");
});

test("missing payload fields do not throw", () => {
  assert.equal(resolveContextProfile(DEFAULT_CONFIG, { profile_id: "" }, DEFAULT_CONFIG), "default");
});

test("wrapIndex wraps in both directions and for multi-step deltas", () => {
  assert.equal(wrapIndex(0, -1, 9), 8);
  assert.equal(wrapIndex(8, 1, 9), 0);
  assert.equal(wrapIndex(3, 1, 9), 4);
  assert.equal(wrapIndex(0, -2, 9), 7); // the old `next < 0 ? count - 1` logic returned 8
  assert.equal(wrapIndex(1, 20, 9), 3);
  assert.equal(wrapIndex(0, 1, 0), 0); // empty wheel must not produce NaN
});
