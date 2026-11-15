/**
 * True inside the Android app, whose WebView shim (`android/.../web/shim.js`) sets this flag
 * before the bundle loads. Desktop (Tauri) and the browser preview never set it.
 */
export function isAndroidApp(): boolean {
  return typeof window !== "undefined" && (window as { __DIAL_PLATFORM__?: string }).__DIAL_PLATFORM__ === "android";
}
