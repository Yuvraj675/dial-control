import { test } from "node:test";
import assert from "node:assert/strict";
import { DEFAULT_CONFIG, sanitizeAndMigrateConfig } from "./defaultProfiles.ts";
import { getOrderedProfileIds } from "../types/profile.ts";
import type { ProfilesConfig } from "../types/profile.ts";

// Must stay in sync with `every_default_profile_key_resolves_on_windows_and_mac` in
// src-tauri/src/platform/keymap.rs, which proves each of these maps on Windows and macOS.
const KEYS_VERIFIED_IN_RUST = new Set([
  "KEY_RIGHT", "KEY_LEFT", "KEY_F", "KEY_M", "KEY_LEFTCTRL", "KEY_S", "KEY_LEFTSHIFT",
  "KEY_LEFTALT", "KEY_SLASH", "KEY_GRAVE", "KEY_G", "KEY_C", "KEY_V", "KEY_DELETE",
  "KEY_N", "KEY_F2", "KEY_R", "KEY_T", "KEY_W", "KEY_Z", "KEY_Y", "KEY_EQUAL",
  "KEY_MINUS", "KEY_H", "KEY_F4", "KEY_V", "KEY_LEFTMETA", "KEY_BACKSPACE", "KEY_ENTER",
  "KEY_LEFTBRACE", "KEY_RIGHTBRACE",
]);

// Backend profile ids returned by match_profile in src-tauri/src/platform/common.rs.
const BACKEND_PROFILE_IDS = ["default", "media_video", "code_editor", "file_manager", "browser_nav", "creative_design"];

test("every hotkey in the defaults is a key the native backends can emit", () => {
  for (const prof of Object.values(DEFAULT_CONFIG.profiles)) {
    for (const action of prof.actions) {
      for (const key of [...(action.keys ?? []), ...(action.keys_mac ?? [])]) {
        assert.ok(KEYS_VERIFIED_IN_RUST.has(key), `${prof.id}/${action.id} uses unverified key ${key}`);
      }
    }
  }
});

test("every profile id the backend can emit exists in the defaults", () => {
  for (const id of BACKEND_PROFILE_IDS) {
    assert.ok(DEFAULT_CONFIG.profiles[id], `missing profile ${id}`);
  }
});

test("action ids are unique within each profile and every profile has HOME and CLOSE", () => {
  const cfg = sanitizeAndMigrateConfig(structuredClone(DEFAULT_CONFIG));
  for (const prof of Object.values(cfg.profiles)) {
    const ids = prof.actions.map((a) => a.id);
    assert.equal(new Set(ids).size, ids.length, `duplicate action id in ${prof.id}`);
    assert.ok(prof.actions.some((a) => a.type === "home"), `${prof.id} lacks HOME`);
    assert.ok(prof.actions.some((a) => a.type === "close"), `${prof.id} lacks CLOSE`);
  }
});

test("sanitize is idempotent (no duplicate slices on repeated launches)", () => {
  const once = sanitizeAndMigrateConfig(structuredClone(DEFAULT_CONFIG));
  const twice = sanitizeAndMigrateConfig(structuredClone(once));
  assert.deepEqual(twice, once);
});

test("sanitize migrates a legacy config without scroll / home / close slices", () => {
  const legacy: ProfilesConfig = structuredClone(DEFAULT_CONFIG);
  for (const prof of Object.values(legacy.profiles)) {
    prof.actions = prof.actions.filter((a) => a.type !== "home" && a.type !== "close" && a.action !== "scroll");
  }
  const migrated = sanitizeAndMigrateConfig(legacy);
  for (const id of ["default", "browser_nav", "code_editor", "file_manager"]) {
    assert.ok(migrated.profiles[id].actions.some((a) => a.action === "scroll"), `${id} lacks SCROLL`);
  }
  for (const prof of Object.values(migrated.profiles)) {
    assert.ok(prof.actions.some((a) => a.type === "home"));
    assert.ok(prof.actions.some((a) => a.type === "close"));
  }
});

test("sanitize falls back to defaults for corrupt configs", () => {
  assert.equal(sanitizeAndMigrateConfig(null as unknown as ProfilesConfig), DEFAULT_CONFIG);
  assert.equal(sanitizeAndMigrateConfig({} as ProfilesConfig), DEFAULT_CONFIG);
});

test("profile order tolerates stale and missing ids", () => {
  const cfg: ProfilesConfig = structuredClone(DEFAULT_CONFIG);
  cfg.profile_order = ["deleted_profile", "code_editor"];
  const ordered = getOrderedProfileIds(cfg);
  assert.equal(ordered[0], "code_editor");
  assert.ok(!ordered.includes("deleted_profile"));
  assert.equal(ordered.length, Object.keys(cfg.profiles).length);
});

test("every PC hotkey slice has a macOS shortcut except identical or media ones", () => {
  const sameOnMac = new Set(["seek_fwd", "seek_bwd", "fullscreen", "video_mute", "draw_hand", "draw_select"]);
  for (const prof of Object.values(DEFAULT_CONFIG.profiles)) {
    for (const a of prof.actions.filter((x) => x.type === "hotkey")) {
      if (sameOnMac.has(a.id)) {
        assert.equal(a.keys_mac, undefined, `${a.id} should reuse its PC shortcut`);
      } else {
        assert.ok(a.keys_mac?.length, `${prof.id}/${a.id} has no macOS shortcut`);
      }
    }
  }
});

test("macOS shortcuts use Command, not Ctrl, except Control-native ones", () => {
  const controlNative = new Set(["code_terminal", "code_git"]);
  for (const prof of Object.values(DEFAULT_CONFIG.profiles)) {
    for (const a of prof.actions.filter((x) => x.keys_mac)) {
      const hasCtrl = a.keys_mac!.includes("KEY_LEFTCTRL");
      assert.equal(hasCtrl, controlNative.has(a.id), `${a.id} Ctrl usage on macOS`);
    }
  }
});

test("redo and browser back/forward differ from the PC shortcuts on macOS", () => {
  const a = (profile: string, id: string) => DEFAULT_CONFIG.profiles[profile].actions.find((x) => x.id === id)!;
  assert.deepEqual(a("creative_design", "draw_redo").keys_mac, ["KEY_LEFTMETA", "KEY_LEFTSHIFT", "KEY_Z"]);
  assert.deepEqual(a("browser_nav", "nav_back").keys_mac, ["KEY_LEFTMETA", "KEY_LEFTBRACE"]);
  assert.deepEqual(a("browser_nav", "nav_fwd").keys_mac, ["KEY_LEFTMETA", "KEY_RIGHTBRACE"]);
});

test("sanitize adds macOS shortcuts to old configs and drops them for customized slices", () => {
  const legacy: ProfilesConfig = structuredClone(DEFAULT_CONFIG);
  for (const prof of Object.values(legacy.profiles)) {
    prof.actions = prof.actions.map(({ keys_mac: _m, ...rest }) => rest);
  }
  // User customised "COPY" to a different PC shortcut.
  legacy.profiles.file_manager.actions.find((x) => x.id === "file_copy")!.keys = ["KEY_LEFTCTRL", "KEY_INSERT"];
  const migrated = sanitizeAndMigrateConfig(legacy);
  const find = (id: string) => migrated.profiles.file_manager.actions.find((x) => x.id === id)!;
  assert.deepEqual(find("file_paste").keys_mac, ["KEY_LEFTMETA", "KEY_V"]);
  assert.equal(find("file_copy").keys_mac, undefined);

  // A stale keys_mac on a customised slice is removed.
  legacy.profiles.file_manager.actions.find((x) => x.id === "file_copy")!.keys_mac = ["KEY_LEFTMETA", "KEY_C"];
  assert.equal(sanitizeAndMigrateConfig(legacy).profiles.file_manager.actions.find((x) => x.id === "file_copy")!.keys_mac, undefined);
});
