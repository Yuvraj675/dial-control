import { test } from "node:test";
import assert from "node:assert/strict";
import { comboToTokens, describeStep, moveItem, newStep, parseKeysInput } from "./macro.ts";

test("combos become evdev tokens", () => {
  assert.deepEqual(comboToTokens("Ctrl+Shift+P"), ["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_P"]);
  assert.deepEqual(comboToTokens("Super+Left"), ["KEY_LEFTMETA", "KEY_LEFT"]);
});

test("free-text key field accepts tokens and combos", () => {
  assert.deepEqual(parseKeysInput("KEY_LEFTALT, KEY_F4"), ["KEY_LEFTALT", "KEY_F4"]);
  assert.deepEqual(parseKeysInput("ctrl+c"), ["KEY_LEFTCTRL", "KEY_C"]);
  assert.deepEqual(parseKeysInput(""), []);
});

test("moveItem moves and clamps", () => {
  assert.deepEqual(moveItem(["a", "b", "c"], 0, 1), ["b", "a", "c"]);
  assert.deepEqual(moveItem(["a", "b", "c"], 2, -1), ["a", "c", "b"]);
  const same = ["a", "b"];
  assert.equal(moveItem(same, 0, -1), same);
  assert.equal(moveItem(same, 1, 1), same);
});

test("new steps carry the fields the backend expects", () => {
  assert.deepEqual(newStep("wait"), { kind: "wait", ms: 500 });
  assert.deepEqual(newStep("hotkey"), { kind: "hotkey", keys: [] });
  assert.deepEqual(newStep("media"), { kind: "media", action: "playpause" });
});

test("step summaries", () => {
  assert.equal(describeStep({ kind: "hotkey", keys: ["KEY_LEFTCTRL", "KEY_S"] }), "CTRL + S");
  assert.equal(describeStep({ kind: "launch", target: "/Applications/Zed.app" }), "Zed.app");
  assert.equal(describeStep({ kind: "launch", target: "C:\\Start Menu\\Spotify.lnk" }), "Spotify.lnk");
  assert.equal(describeStep({ kind: "wait", ms: 300 }), "300 MS");
});
