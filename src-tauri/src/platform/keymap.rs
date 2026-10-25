//! Translation of profile key names (evdev-style `KEY_LEFTCTRL`, `KEY_S`, or bare `Ctrl`, `S`)
//! into native key codes. Kept platform-agnostic so every mapping is unit-tested on any host.

fn normalize(name: &str) -> String {
    let upper = name.trim().to_uppercase();
    upper.strip_prefix("KEY_").map(str::to_string).unwrap_or(upper)
}

/// Windows virtual-key code for a profile key name.
pub fn windows_vk(key_name: &str) -> Option<u16> {
    let clean = normalize(key_name);
    match clean.as_str() {
        "CTRL" | "CONTROL" | "LEFTCTRL" | "RIGHTCTRL" | "LCTRL" | "RCTRL" => Some(0x11), // VK_CONTROL
        "SHIFT" | "LEFTSHIFT" | "RIGHTSHIFT" | "LSHIFT" | "RSHIFT" => Some(0x10), // VK_SHIFT
        "ALT" | "MENU" | "LEFTALT" | "RIGHTALT" | "LALT" | "RALT" => Some(0x12), // VK_MENU
        "SUPER" | "WIN" | "META" | "COMMAND" | "LEFTMETA" | "RIGHTMETA" => Some(0x5B), // VK_LWIN
        "ENTER" | "RETURN" => Some(0x0D),
        "SPACE" | "SPACEBAR" => Some(0x20),
        "TAB" => Some(0x09),
        "BACKSPACE" | "BACK" => Some(0x08),
        "DELETE" | "DEL" => Some(0x2E),
        "ESCAPE" | "ESC" => Some(0x1B),
        "LEFT" => Some(0x25),
        "UP" => Some(0x26),
        "RIGHT" => Some(0x27),
        "DOWN" => Some(0x28),
        "PAGEUP" | "PAGE_UP" => Some(0x21),
        "PAGEDOWN" | "PAGE_DOWN" => Some(0x22),
        "END" => Some(0x23),
        "HOME" => Some(0x24),
        "INSERT" => Some(0x2D),
        "SLASH" => Some(0xBF),                       // VK_OEM_2
        "BACKSLASH" => Some(0xDC),                   // VK_OEM_5
        "GRAVE" | "BACKTICK" => Some(0xC0),          // VK_OEM_3
        "MINUS" => Some(0xBD),                       // VK_OEM_MINUS
        "EQUAL" | "PLUS" => Some(0xBB),              // VK_OEM_PLUS
        "LEFTBRACE" | "BRACKETLEFT" => Some(0xDB),   // VK_OEM_4
        "RIGHTBRACE" | "BRACKETRIGHT" => Some(0xDD), // VK_OEM_6
        "SEMICOLON" => Some(0xBA),                   // VK_OEM_1
        "APOSTROPHE" | "QUOTE" => Some(0xDE),        // VK_OEM_7
        "COMMA" => Some(0xBC),                       // VK_OEM_COMMA
        "DOT" | "PERIOD" => Some(0xBE),              // VK_OEM_PERIOD
        s if s.len() >= 2 && s.starts_with('F') && s[1..].parse::<u16>().is_ok() => {
            let n: u16 = s[1..].parse().ok()?;
            if (1..=24).contains(&n) {
                Some(0x70 + n - 1)
            } else {
                None
            }
        }
        s if s.chars().count() == 1 => {
            let c = s.chars().next()?;
            if c.is_ascii_alphanumeric() {
                Some(c as u16)
            } else {
                match c {
                    '`' | '~' => Some(0xC0),
                    '-' | '_' => Some(0xBD),
                    '=' | '+' => Some(0xBB),
                    '[' | '{' => Some(0xDB),
                    ']' | '}' => Some(0xDD),
                    '\\' | '|' => Some(0xDC),
                    ';' | ':' => Some(0xBA),
                    '\'' | '"' => Some(0xDE),
                    ',' | '<' => Some(0xBC),
                    '.' | '>' => Some(0xBE),
                    '/' | '?' => Some(0xBF),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}

pub const MAC_FLAG_SHIFT: u64 = 0x0002_0000;
pub const MAC_FLAG_CONTROL: u64 = 0x0004_0000;
pub const MAC_FLAG_OPTION: u64 = 0x0008_0000;
pub const MAC_FLAG_COMMAND: u64 = 0x0010_0000;

/// macOS virtual keycode (ANSI layout, `kVK_*` from HIToolbox/Events.h).
pub fn mac_keycode(key_name: &str) -> Option<u16> {
    let clean = normalize(key_name);
    let code = match clean.as_str() {
        "A" => 0x00, "S" => 0x01, "D" => 0x02, "F" => 0x03, "H" => 0x04, "G" => 0x05,
        "Z" => 0x06, "X" => 0x07, "C" => 0x08, "V" => 0x09, "B" => 0x0B, "Q" => 0x0C,
        "W" => 0x0D, "E" => 0x0E, "R" => 0x0F, "Y" => 0x10, "T" => 0x11, "O" => 0x1F,
        "U" => 0x20, "I" => 0x22, "P" => 0x23, "L" => 0x25, "J" => 0x26, "K" => 0x28,
        "N" => 0x2D, "M" => 0x2E,
        "1" => 0x12, "2" => 0x13, "3" => 0x14, "4" => 0x15, "5" => 0x17, "6" => 0x16,
        "7" => 0x1A, "8" => 0x1C, "9" => 0x19, "0" => 0x1D,
        "EQUAL" | "=" => 0x18,
        "MINUS" | "-" => 0x1B,
        "RIGHTBRACE" | "]" => 0x1E,
        "LEFTBRACE" | "[" => 0x21,
        "APOSTROPHE" | "'" => 0x27,
        "SEMICOLON" | ";" => 0x29,
        "BACKSLASH" | "\\" => 0x2A,
        "COMMA" | "," => 0x2B,
        "SLASH" | "/" => 0x2C,
        "DOT" | "PERIOD" | "." => 0x2F,
        "GRAVE" | "`" => 0x32,
        "ENTER" | "RETURN" => 0x24,
        "TAB" => 0x30,
        "SPACE" => 0x31,
        "BACKSPACE" => 0x33,
        "ESC" | "ESCAPE" => 0x35,
        // PC "Delete" is forward-delete on a Mac keyboard.
        "DELETE" | "DEL" => 0x75,
        "INSERT" => 0x72, // kVK_Help occupies the Insert position
        "HOME" => 0x73,
        "PAGEUP" | "PAGE_UP" => 0x74,
        "END" => 0x77,
        "PAGEDOWN" | "PAGE_DOWN" => 0x79,
        "LEFT" | "ARROWLEFT" => 0x7B,
        "RIGHT" | "ARROWRIGHT" => 0x7C,
        "DOWN" | "ARROWDOWN" => 0x7D,
        "UP" | "ARROWUP" => 0x7E,
        "F1" => 0x7A, "F2" => 0x78, "F3" => 0x63, "F4" => 0x76, "F5" => 0x60, "F6" => 0x61,
        "F7" => 0x62, "F8" => 0x64, "F9" => 0x65, "F10" => 0x6D, "F11" => 0x67, "F12" => 0x6F,
        _ => return None,
    };
    Some(code)
}

/// Resolves a profile hotkey into `(CGEventFlags, keycode)` for macOS.
///
/// Profiles are authored with PC conventions (`Ctrl+C`, `Ctrl+S`). On macOS the equivalent
/// primary modifier is Command, so Ctrl is translated to Command — except for the few
/// shortcuts that genuinely use Control on a Mac too (Ctrl+` terminal, Ctrl+Tab tabs).
pub fn mac_hotkey(keys: &[String]) -> Option<(u64, u16)> {
    let mut wants_ctrl = false;
    let mut flags: u64 = 0;
    let mut keycode: Option<u16> = None;
    let mut key_name = String::new();

    for key in keys {
        match normalize(key).as_str() {
            "CTRL" | "CONTROL" | "LEFTCTRL" | "RIGHTCTRL" | "LCTRL" | "RCTRL" => wants_ctrl = true,
            "SHIFT" | "LEFTSHIFT" | "RIGHTSHIFT" | "LSHIFT" | "RSHIFT" => flags |= MAC_FLAG_SHIFT,
            "ALT" | "OPTION" | "OPT" | "LEFTALT" | "RIGHTALT" | "LALT" | "RALT" => flags |= MAC_FLAG_OPTION,
            "SUPER" | "META" | "WIN" | "CMD" | "COMMAND" | "LEFTMETA" | "RIGHTMETA" => {
                flags |= MAC_FLAG_COMMAND
            }
            other => {
                if let Some(code) = mac_keycode(other) {
                    keycode = Some(code);
                    key_name = other.to_string();
                }
            }
        }
    }

    if wants_ctrl {
        let control_native = matches!(key_name.as_str(), "GRAVE" | "`" | "TAB");
        flags |= if control_native { MAC_FLAG_CONTROL } else { MAC_FLAG_COMMAND };
    }

    keycode.map(|code| (flags, code))
}

/// Resolves an explicit macOS shortcut (`keys_mac`) exactly as written: Ctrl stays Control
/// and Meta/Super is Command. No PC-to-Mac translation is applied.
pub fn mac_hotkey_literal(keys: &[String]) -> Option<(u64, u16)> {
    let mut flags: u64 = 0;
    let mut keycode: Option<u16> = None;
    for key in keys {
        match normalize(key).as_str() {
            "CTRL" | "CONTROL" | "LEFTCTRL" | "RIGHTCTRL" | "LCTRL" | "RCTRL" => flags |= MAC_FLAG_CONTROL,
            "SHIFT" | "LEFTSHIFT" | "RIGHTSHIFT" | "LSHIFT" | "RSHIFT" => flags |= MAC_FLAG_SHIFT,
            "ALT" | "OPTION" | "OPT" | "LEFTALT" | "RIGHTALT" | "LALT" | "RALT" => flags |= MAC_FLAG_OPTION,
            "SUPER" | "META" | "WIN" | "CMD" | "COMMAND" | "LEFTMETA" | "RIGHTMETA" => flags |= MAC_FLAG_COMMAND,
            other => {
                if let Some(code) = mac_keycode(other) {
                    keycode = Some(code);
                }
            }
        }
    }
    keycode.map(|code| (flags, code))
}

/// Linux evdev key code and Shift state that type `c` on a US keyboard layout. Used to type
/// text through uinput, which only sends key positions; other layouts need `wtype`/`xdotool`.
pub fn linux_us_char_key(c: char) -> Option<(u16, bool)> {
    const LETTERS: [u16; 26] = [
        30, 48, 46, 32, 18, 33, 34, 35, 23, 36, 37, 38, 50, 49, 24, 25, 16, 19, 31, 20, 22, 47, 17, 45, 21, 44,
    ];
    let key = match c {
        'a'..='z' => (LETTERS[(c as u8 - b'a') as usize], false),
        'A'..='Z' => (LETTERS[(c as u8 - b'A') as usize], true),
        '1'..='9' => ((c as u8 - b'1') as u16 + 2, false),
        '0' => (11, false),
        '!' => (2, true),
        '@' => (3, true),
        '#' => (4, true),
        '$' => (5, true),
        '%' => (6, true),
        '^' => (7, true),
        '&' => (8, true),
        '*' => (9, true),
        '(' => (10, true),
        ')' => (11, true),
        '-' => (12, false),
        '_' => (12, true),
        '=' => (13, false),
        '+' => (13, true),
        '\t' => (15, false),
        '[' => (26, false),
        '{' => (26, true),
        ']' => (27, false),
        '}' => (27, true),
        '\n' => (28, false),
        ';' => (39, false),
        ':' => (39, true),
        '\'' => (40, false),
        '"' => (40, true),
        '`' => (41, false),
        '~' => (41, true),
        '\\' => (43, false),
        '|' => (43, true),
        ',' => (51, false),
        '<' => (51, true),
        '.' => (52, false),
        '>' => (52, true),
        '/' => (53, false),
        '?' => (53, true),
        ' ' => (57, false),
        _ => return None,
    };
    Some(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn windows_parses_evdev_style_names() {
        assert_eq!(windows_vk("KEY_LEFTCTRL"), Some(0x11));
        assert_eq!(windows_vk("KEY_LEFTSHIFT"), Some(0x10));
        assert_eq!(windows_vk("KEY_LEFTALT"), Some(0x12));
        assert_eq!(windows_vk("KEY_LEFTMETA"), Some(0x5B));
        assert_eq!(windows_vk("KEY_S"), Some(b'S' as u16));
        assert_eq!(windows_vk("key_s"), Some(b'S' as u16));
        assert_eq!(windows_vk("KEY_7"), Some(b'7' as u16));
        assert_eq!(windows_vk("KEY_GRAVE"), Some(0xC0));
        assert_eq!(windows_vk("KEY_SLASH"), Some(0xBF));
        assert_eq!(windows_vk("KEY_F2"), Some(0x71));
        assert_eq!(windows_vk("KEY_F12"), Some(0x7B));
        assert_eq!(windows_vk("KEY_DELETE"), Some(0x2E));
        assert_eq!(windows_vk("KEY_INSERT"), Some(0x2D));
        assert_eq!(windows_vk("KEY_PAGEDOWN"), Some(0x22));
    }

    #[test]
    fn windows_rejects_unknown_names() {
        assert_eq!(windows_vk("KEY_NOPE"), None);
        assert_eq!(windows_vk(""), None);
        assert_eq!(windows_vk("KEY_F0"), None);
        assert_eq!(windows_vk("KEY_F99"), None);
        assert_eq!(windows_vk("€"), None);
    }

    #[test]
    fn every_default_profile_key_resolves_on_windows_and_mac() {
        // Every key token used by src/data/defaultProfiles.ts.
        let used = [
            "KEY_RIGHT", "KEY_LEFT", "KEY_F", "KEY_M", "KEY_LEFTCTRL", "KEY_S", "KEY_LEFTSHIFT",
            "KEY_LEFTALT", "KEY_SLASH", "KEY_GRAVE", "KEY_G", "KEY_C", "KEY_V", "KEY_DELETE",
            "KEY_N", "KEY_F2", "KEY_R", "KEY_T", "KEY_W", "KEY_Z", "KEY_Y", "KEY_EQUAL",
            "KEY_MINUS", "KEY_H", "KEY_F4", "KEY_V", "KEY_LEFTMETA", "KEY_BACKSPACE", "KEY_ENTER",
            "KEY_LEFTBRACE", "KEY_RIGHTBRACE",
        ];
        for k in used {
            assert!(windows_vk(k).is_some(), "Windows cannot map {k}");
            let is_modifier = matches!(k, "KEY_LEFTCTRL" | "KEY_LEFTSHIFT" | "KEY_LEFTALT" | "KEY_LEFTMETA");
            if !is_modifier {
                assert!(mac_keycode(k).is_some(), "macOS cannot map {k}");
            }
        }
    }

    #[test]
    fn every_default_macos_shortcut_resolves() {
        // keys_mac lists from src/data/defaultProfiles.ts; each must produce a real keycode.
        let shortcuts: &[&[&str]] = &[
            &["KEY_LEFTMETA", "KEY_S"], &["KEY_LEFTSHIFT", "KEY_LEFTALT", "KEY_F"],
            &["KEY_LEFTMETA", "KEY_LEFTSHIFT", "KEY_F"], &["KEY_LEFTMETA", "KEY_SLASH"],
            &["KEY_LEFTCTRL", "KEY_GRAVE"], &["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_G"],
            &["KEY_LEFTMETA", "KEY_C"], &["KEY_LEFTMETA", "KEY_V"], &["KEY_LEFTMETA", "KEY_BACKSPACE"],
            &["KEY_LEFTMETA", "KEY_LEFTSHIFT", "KEY_N"], &["KEY_ENTER"],
            &["KEY_LEFTMETA", "KEY_LEFTBRACE"], &["KEY_LEFTMETA", "KEY_RIGHTBRACE"],
            &["KEY_LEFTMETA", "KEY_R"], &["KEY_LEFTMETA", "KEY_T"], &["KEY_LEFTMETA", "KEY_W"],
            &["KEY_LEFTMETA", "KEY_LEFTSHIFT", "KEY_T"], &["KEY_LEFTMETA", "KEY_Z"],
            &["KEY_LEFTMETA", "KEY_LEFTSHIFT", "KEY_Z"], &["KEY_LEFTMETA", "KEY_EQUAL"],
            &["KEY_LEFTMETA", "KEY_MINUS"],
        ];
        for sc in shortcuts {
            assert!(mac_hotkey_literal(&keys(sc)).is_some(), "unresolvable macOS shortcut {sc:?}");
        }
    }

    #[test]
    fn mac_regression_evdev_names_were_silently_dropped() {
        // Previously "KEY_LEFTCTRL"/"KEY_S" matched nothing, so Save did nothing on macOS.
        assert_eq!(mac_hotkey(&keys(&["KEY_LEFTCTRL", "KEY_S"])), Some((MAC_FLAG_COMMAND, 0x01)));
    }

    #[test]
    fn mac_translates_ctrl_to_command() {
        assert_eq!(mac_hotkey(&keys(&["KEY_LEFTCTRL", "KEY_C"])), Some((MAC_FLAG_COMMAND, 0x08)));
        assert_eq!(
            mac_hotkey(&keys(&["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_T"])),
            Some((MAC_FLAG_COMMAND | MAC_FLAG_SHIFT, 0x11))
        );
        assert_eq!(
            mac_hotkey(&keys(&["KEY_LEFTSHIFT", "KEY_LEFTALT", "KEY_F"])),
            Some((MAC_FLAG_SHIFT | MAC_FLAG_OPTION, 0x03))
        );
    }

    #[test]
    fn mac_keeps_control_where_macs_use_control() {
        assert_eq!(mac_hotkey(&keys(&["KEY_LEFTCTRL", "KEY_GRAVE"])), Some((MAC_FLAG_CONTROL, 0x32)));
        assert_eq!(mac_hotkey(&keys(&["Ctrl", "Tab"])), Some((MAC_FLAG_CONTROL, 0x30)));
    }

    #[test]
    fn mac_literal_keeps_control_and_maps_meta_to_command() {
        assert_eq!(
            mac_hotkey_literal(&keys(&["KEY_LEFTCTRL", "KEY_LEFTSHIFT", "KEY_G"])),
            Some((MAC_FLAG_CONTROL | MAC_FLAG_SHIFT, 0x05))
        );
        assert_eq!(mac_hotkey_literal(&keys(&["KEY_LEFTMETA", "KEY_C"])), Some((MAC_FLAG_COMMAND, 0x08)));
        assert_eq!(mac_hotkey_literal(&keys(&["KEY_LEFTMETA", "KEY_LEFTBRACE"])), Some((MAC_FLAG_COMMAND, 0x21)));
        assert_eq!(mac_hotkey_literal(&keys(&["KEY_LEFTMETA", "KEY_BACKSPACE"])), Some((MAC_FLAG_COMMAND, 0x33)));
        assert_eq!(mac_hotkey_literal(&keys(&["KEY_ENTER"])), Some((0, 0x24)));
        assert_eq!(mac_hotkey_literal(&keys(&["KEY_LEFTMETA"])), None);
    }

    #[test]
    fn mac_bare_and_navigation_keys() {
        assert_eq!(mac_hotkey(&keys(&["KEY_RIGHT"])), Some((0, 0x7C)));
        assert_eq!(mac_hotkey(&keys(&["KEY_DELETE"])), Some((0, 0x75)));
        assert_eq!(mac_hotkey(&keys(&["KEY_LEFTMETA", "KEY_Q"])), Some((MAC_FLAG_COMMAND, 0x0C)));
    }

    #[test]
    fn mac_modifier_only_or_unknown_yields_none() {
        assert_eq!(mac_hotkey(&keys(&["KEY_LEFTCTRL"])), None);
        assert_eq!(mac_hotkey(&keys(&[])), None);
        assert_eq!(mac_hotkey(&keys(&["KEY_PLAYPAUSE"])), None);
    }

    #[test]
    fn us_layout_characters() {
        assert_eq!(linux_us_char_key('a'), Some((30, false)));
        assert_eq!(linux_us_char_key('Z'), Some((44, true)));
        assert_eq!(linux_us_char_key('1'), Some((2, false)));
        assert_eq!(linux_us_char_key('0'), Some((11, false)));
        assert_eq!(linux_us_char_key('@'), Some((3, true)));
        assert_eq!(linux_us_char_key(' '), Some((57, false)));
        assert_eq!(linux_us_char_key('\n'), Some((28, false)));
        assert_eq!(linux_us_char_key('?'), Some((53, true)));
        assert_eq!(linux_us_char_key('é'), None);
    }
}
