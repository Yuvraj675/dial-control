//! Shows that scroll mode is active without opening a window over the app being scrolled
//! (floating pills and overlays stole focus on Wayland, see CLAUDE.md §6):
//! a red tray icon with an "Exit Scroll Mode" item, a GNOME top-bar pill via the bundled
//! Shell extension, and a short tone on entry and exit.

use crate::platform::cue::{self, Cue};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{image::Image, menu::MenuItem, AppHandle, Manager, Wry};

pub const TRAY_ID: &str = "main_tray";
pub const EXIT_MENU_ID: &str = "exit_scroll";
pub const TOOLTIP_IDLE: &str = "Dial Control — Active";
const TOOLTIP_SCROLL: &str = "Dial Control — Scroll mode (click the dial to exit)";

/// Nothing red, used for the scroll-mode tray icon.
const SCROLL_TINT: [u8; 3] = [215, 25, 33];

static ACTIVE: AtomicBool = AtomicBool::new(false);

/// Tray menu item enabled only while scroll mode is on.
pub struct ExitScrollItem(pub MenuItem<Wry>);

/// Recolours the lighter parts of the icon (knob face, ring and ticks) red, keeping the dark body and
/// alpha, so the scroll state is recognisable even at tray size.
pub fn scroll_variant(rgba: &[u8]) -> Vec<u8> {
    let mut out = rgba.to_vec();
    for px in out.chunks_exact_mut(4) {
        let luma = (px[0] as u32 * 299 + px[1] as u32 * 587 + px[2] as u32 * 114) / 1000;
        // The knob face is a dark grey gradient; doubling its brightness makes it read as red
        // even at 22px tray size.
        if px[3] > 0 && luma > 38 {
            let k = (luma * 2).min(255);
            for c in 0..3 {
                px[c] = (SCROLL_TINT[c] as u32 * k / 255).max(SCROLL_TINT[c] as u32 / 2) as u8;
            }
        }
    }
    out
}

/// Idempotent: entry and the several exit paths (click, gesture, shortcut, HUD reopen) may all
/// call it, but the tone and the D-Bus call only happen on a real change.
pub fn set(app: &AppHandle, on: bool) {
    if ACTIVE.swap(on, Ordering::SeqCst) == on {
        return;
    }
    crate::platform::diag::log(&format!("scroll indicator {}", if on { "on" } else { "off" }));
    cue::play(if on { Cue::ScrollOn } else { Cue::ScrollOff });

    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        if let Some(icon) = app.default_window_icon() {
            let icon = if on {
                Image::new_owned(scroll_variant(icon.rgba()), icon.width(), icon.height())
            } else {
                icon.clone()
            };
            let _ = tray.set_icon(Some(icon));
        }
        let _ = tray.set_tooltip(Some(if on { TOOLTIP_SCROLL } else { TOOLTIP_IDLE }));
    }
    if let Some(item) = app.try_state::<ExitScrollItem>() {
        let _ = item.0.set_enabled(on);
    }

    #[cfg(target_os = "linux")]
    std::thread::spawn(move || set_gnome_pill(on));
}

/// Top-bar pill drawn by the bundled GNOME Shell extension. Silently does nothing on other
/// desktops, or until GNOME Shell has loaded the updated extension (next login on Wayland).
#[cfg(target_os = "linux")]
fn set_gnome_pill(on: bool) {
    let Ok(conn) = zbus::blocking::Connection::session() else {
        return;
    };
    let result = conn.call_method(
        Some("org.gnome.Shell"),
        "/org/gnome/shell/extensions/FocusedWindow",
        Some("org.gnome.shell.extensions.FocusedWindow"),
        "SetScrollMode",
        &(on,),
    );
    if let Err(e) = result {
        println!("[Scroll Indicator] GNOME top-bar pill unavailable: {e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn light_pixels_turn_red_dark_and_transparent_stay() {
        let rgba = [
            230, 230, 230, 255, // light ring
            20, 20, 20, 255, // dark body
            255, 255, 255, 0, // transparent corner
        ];
        let out = scroll_variant(&rgba);
        assert!(out[0] > 150 && out[1] < 60 && out[2] < 60, "ring should be red: {:?}", &out[0..4]);
        assert_eq!(out[3], 255);
        assert_eq!(&out[4..8], &rgba[4..8]);
        assert_eq!(&out[8..12], &rgba[8..12]);
    }
}
