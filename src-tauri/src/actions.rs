//! Custom slice actions: hotkeys, app launches, shell commands, typed text and macros.
//!
//! Every such slice is reduced to a list of [`Step`]s, so a single hotkey slice and a
//! ten-step macro run through the same executor. The OS-specific parts (how to launch a
//! target, how to run a command) are pure functions of the target platform, so the
//! Windows and macOS behaviour is unit-tested on any host.

use crate::platform::{Platform, PlatformProvider};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;

/// Longest single wait a macro may ask for, so a typo cannot hang the executor.
pub const MAX_WAIT_MS: u64 = 10_000;
/// Pause between steps so the focused app can process the previous one.
const STEP_GAP_MS: u64 = 40;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Step {
    Hotkey {
        #[serde(default)]
        keys: Vec<String>,
        /// Same shortcut authored for macOS (Meta = Command), used as written.
        #[serde(default)]
        keys_mac: Option<Vec<String>>,
    },
    Text {
        #[serde(default)]
        text: String,
    },
    Wait {
        #[serde(default)]
        ms: u64,
    },
    Launch {
        #[serde(default)]
        target: String,
    },
    Command {
        #[serde(default)]
        cmd: String,
    },
    Media {
        #[serde(default)]
        action: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Windows,
    MacOs,
}

pub const HOST_OS: Os = if cfg!(target_os = "windows") {
    Os::Windows
} else if cfg!(target_os = "macos") {
    Os::MacOs
} else {
    Os::Linux
};

fn str_list(v: Option<&serde_json::Value>) -> Option<Vec<String>> {
    v.and_then(|k| k.as_array())
        .map(|arr| arr.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
}

/// The steps a slice runs, or `None` for slice types handled elsewhere (modes, media, ...).
pub fn steps_for(action: &serde_json::Value) -> Option<Vec<Step>> {
    let field = |name: &str| action.get(name).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let steps = match action.get("type").and_then(|v| v.as_str()).unwrap_or("") {
        "hotkey" => vec![Step::Hotkey {
            keys: str_list(action.get("keys")).unwrap_or_default(),
            keys_mac: str_list(action.get("keys_mac")),
        }],
        "launch" => vec![Step::Launch { target: field("target") }],
        "command" => vec![Step::Command { cmd: field("cmd") }],
        "text" => vec![Step::Text { text: field("text") }],
        // A malformed step is skipped rather than failing the whole macro.
        "macro" => action
            .get("steps")
            .and_then(|s| s.as_array())
            .map(|arr| arr.iter().filter_map(|s| serde_json::from_value(s.clone()).ok()).collect())
            .unwrap_or_default(),
        _ => return None,
    };
    Some(steps)
}

/// Program and arguments that open `target`: an installed app (as listed by
/// [`list_installed_apps`]), an executable, a file, a folder or a URL.
pub fn launch_invocation(target: &str, os: Os) -> Option<Vec<String>> {
    let target = target.trim();
    if target.is_empty() {
        return None;
    }
    let owned = |parts: &[&str]| parts.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let is_url = target.contains("://") || target.starts_with("mailto:");
    Some(match os {
        Os::Linux => {
            if let Some(id) = target.strip_suffix(".desktop") {
                // gtk-launch takes the desktop id, never a path.
                let id = id.rsplit('/').next().unwrap_or(id);
                owned(&["gtk-launch", id])
            } else if is_url || target.starts_with('/') || target.starts_with('~') {
                owned(&["xdg-open", target])
            } else {
                owned(&["sh", "-c", target])
            }
        }
        // `start` resolves Start Menu shortcuts, App Paths names, files and URLs; the empty
        // string is the window title, which `start` would otherwise take from a quoted target.
        Os::Windows => owned(&["cmd", "/c", "start", "", target]),
        Os::MacOs => {
            if is_url || target.starts_with('/') || target.starts_with('~') || target.ends_with(".app") {
                owned(&["open", target])
            } else {
                owned(&["open", "-a", target])
            }
        }
    })
}

/// Program and arguments that run a shell command line without a console window.
pub fn command_invocation(cmd: &str, os: Os) -> Option<Vec<String>> {
    let cmd = cmd.trim();
    if cmd.is_empty() {
        return None;
    }
    Some(match os {
        Os::Windows => vec!["cmd".into(), "/c".into(), cmd.into()],
        Os::Linux | Os::MacOs => vec!["sh".into(), "-c".into(), cmd.into()],
    })
}

fn spawn(argv: &[String]) {
    let Some((program, args)) = argv.split_first() else {
        return;
    };
    let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).ok();
    let mut proc = std::process::Command::new(program);
    // Paths like ~/Notes.md reach xdg-open / open without a shell to expand them.
    proc.args(args.iter().map(|a| match (a.strip_prefix("~/"), &home) {
        (Some(rest), Some(h)) => format!("{h}/{rest}"),
        _ => a.clone(),
    }));
    if let Some(h) = &home {
        proc.current_dir(h);
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        proc.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    match proc.spawn() {
        // Reap the child in the background so finished commands do not linger as zombies.
        Ok(mut child) => {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
        Err(e) => eprintln!("[Action] Could not start {:?}: {}", argv, e),
    }
}

/// Runs the steps in order on the calling thread (callers spawn a worker).
pub fn run(platform: &Arc<Platform>, steps: &[Step]) {
    for (i, step) in steps.iter().enumerate() {
        if i > 0 {
            std::thread::sleep(Duration::from_millis(STEP_GAP_MS));
        }
        crate::platform::diag::log(&format!("action step {}: {:?}", i + 1, step));
        match step {
            Step::Hotkey { keys, keys_mac } => match keys_mac {
                Some(native) if cfg!(target_os = "macos") && !native.is_empty() => {
                    platform.input().emit_native_hotkey(native)
                }
                _ if !keys.is_empty() => platform.input().emit_hotkey(keys),
                _ => {}
            },
            Step::Text { text } if !text.is_empty() => platform.input().type_text(text),
            Step::Text { .. } => {}
            Step::Wait { ms } => std::thread::sleep(Duration::from_millis((*ms).min(MAX_WAIT_MS))),
            Step::Launch { target } => {
                if let Some(argv) = launch_invocation(target, HOST_OS) {
                    spawn(&argv);
                }
            }
            Step::Command { cmd } => {
                if let Some(argv) = command_invocation(cmd, HOST_OS) {
                    spawn(&argv);
                }
            }
            Step::Media { action } => match action.as_str() {
                "playpause" => platform.media().play_pause(),
                "next" => platform.media().next_track(),
                "prev" => platform.media().prev_track(),
                _ => {}
            },
        }
    }
}

// ── Installed applications (for the Settings app picker) ─────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstalledApp {
    pub name: String,
    /// Value for a launch step: a desktop id (Linux), a Start Menu shortcut (Windows) or an
    /// .app bundle path (macOS).
    pub target: String,
}

/// Visible application name from a freedesktop `.desktop` file, or `None` when the entry is
/// hidden, not an application, or has no name.
pub fn parse_desktop_entry(contents: &str) -> Option<String> {
    let mut in_entry = false;
    let mut name = None;
    let mut is_app = false;
    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        match line.split_once('=').map(|(k, v)| (k.trim(), v.trim())) {
            Some(("Name", v)) if name.is_none() => name = Some(v.to_string()),
            Some(("Type", v)) => is_app = v == "Application",
            Some(("NoDisplay" | "Hidden", "true")) => return None,
            _ => {}
        }
    }
    name.filter(|n| is_app && !n.is_empty())
}

fn collect_dir(dir: &std::path::Path, depth: u32, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "app") {
            out.push(path); // macOS bundles are directories; never descend into them
        } else if path.is_dir() && depth > 0 {
            collect_dir(&path, depth - 1, out);
        } else {
            out.push(path);
        }
    }
}

pub fn list_installed_apps() -> Vec<InstalledApp> {
    let home = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")).unwrap_or_default();
    let mut files = Vec::new();
    let mut apps: Vec<InstalledApp> = Vec::new();

    if HOST_OS == Os::Linux {
        let mut dirs: Vec<String> = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".into())
            .split(':')
            .map(|d| format!("{d}/applications"))
            .collect();
        dirs.push(format!("{home}/.local/share/applications"));
        dirs.push("/var/lib/flatpak/exports/share/applications".into());
        dirs.push(format!("{home}/.local/share/flatpak/exports/share/applications"));
        for d in dirs {
            collect_dir(std::path::Path::new(&d), 1, &mut files);
        }
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "desktop")) {
            let (Ok(text), Some(file_name)) = (std::fs::read_to_string(f), f.file_name()) else {
                continue;
            };
            if let Some(name) = parse_desktop_entry(&text) {
                apps.push(InstalledApp { name, target: file_name.to_string_lossy().into_owned() });
            }
        }
    } else if HOST_OS == Os::Windows {
        let roots = [
            std::env::var("ProgramData").map(|p| format!("{p}\\Microsoft\\Windows\\Start Menu\\Programs")),
            std::env::var("APPDATA").map(|p| format!("{p}\\Microsoft\\Windows\\Start Menu\\Programs")),
        ];
        for root in roots.into_iter().flatten() {
            collect_dir(std::path::Path::new(&root), 3, &mut files);
        }
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk"))) {
            let name = f.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            // Skip uninstallers and help links that clutter every Start Menu folder.
            let lower = name.to_lowercase();
            if lower.contains("uninstall") || lower.ends_with(" help") || lower.contains("readme") {
                continue;
            }
            apps.push(InstalledApp { name, target: f.to_string_lossy().into_owned() });
        }
    } else {
        for d in ["/Applications", "/Applications/Utilities", "/System/Applications", "/System/Applications/Utilities"]
            .iter()
            .map(|d| d.to_string())
            .chain([format!("{home}/Applications")])
        {
            collect_dir(std::path::Path::new(&d), 0, &mut files);
        }
        for f in files.iter().filter(|f| f.extension().is_some_and(|e| e == "app")) {
            let name = f.file_stem().unwrap_or_default().to_string_lossy().into_owned();
            apps.push(InstalledApp { name, target: f.to_string_lossy().into_owned() });
        }
    }

    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps.dedup_by(|a, b| a.name.eq_ignore_ascii_case(&b.name));
    apps
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn simple_slices_become_one_step() {
        assert_eq!(
            steps_for(&json!({"type": "hotkey", "keys": ["CTRL", "C"]})),
            Some(vec![Step::Hotkey { keys: vec!["CTRL".into(), "C".into()], keys_mac: None }])
        );
        assert_eq!(
            steps_for(&json!({"type": "launch", "target": "firefox.desktop"})),
            Some(vec![Step::Launch { target: "firefox.desktop".into() }])
        );
        assert_eq!(steps_for(&json!({"type": "text", "text": "hi"})), Some(vec![Step::Text { text: "hi".into() }]));
        assert_eq!(steps_for(&json!({"type": "mode", "action": "scroll"})), None);
    }

    #[test]
    fn macro_steps_parse_and_bad_steps_are_skipped() {
        let action = json!({"type": "macro", "steps": [
            {"kind": "launch", "target": "code"},
            {"kind": "wait", "ms": 800},
            {"kind": "bogus"},
            {"kind": "hotkey", "keys": ["CTRL", "SHIFT", "P"], "keys_mac": ["META", "SHIFT", "P"]},
            {"kind": "text", "text": "format"},
            {"kind": "media", "action": "next"}
        ]});
        let steps = steps_for(&action).unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[1], Step::Wait { ms: 800 });
        assert!(matches!(&steps[2], Step::Hotkey { keys_mac: Some(m), .. } if m[0] == "META"));
        assert_eq!(steps_for(&json!({"type": "macro"})), Some(vec![]));
    }

    #[test]
    fn launch_on_linux() {
        assert_eq!(
            launch_invocation("org.mozilla.firefox.desktop", Os::Linux).unwrap(),
            ["gtk-launch", "org.mozilla.firefox"]
        );
        assert_eq!(
            launch_invocation("/usr/share/applications/code.desktop", Os::Linux).unwrap(),
            ["gtk-launch", "code"]
        );
        assert_eq!(launch_invocation("https://x.com", Os::Linux).unwrap(), ["xdg-open", "https://x.com"]);
        assert_eq!(launch_invocation("~/Notes.md", Os::Linux).unwrap(), ["xdg-open", "~/Notes.md"]);
        assert_eq!(launch_invocation("spotify --minimized", Os::Linux).unwrap(), ["sh", "-c", "spotify --minimized"]);
        assert_eq!(launch_invocation("   ", Os::Linux), None);
    }

    #[test]
    fn launch_on_windows_uses_start_with_empty_title() {
        assert_eq!(
            launch_invocation(r"C:\ProgramData\Start Menu\Spotify.lnk", Os::Windows).unwrap(),
            ["cmd", "/c", "start", "", r"C:\ProgramData\Start Menu\Spotify.lnk"]
        );
    }

    #[test]
    fn launch_on_macos() {
        assert_eq!(launch_invocation("Safari", Os::MacOs).unwrap(), ["open", "-a", "Safari"]);
        assert_eq!(
            launch_invocation("/Applications/Zed.app", Os::MacOs).unwrap(),
            ["open", "/Applications/Zed.app"]
        );
        assert_eq!(launch_invocation("https://x.com", Os::MacOs).unwrap(), ["open", "https://x.com"]);
    }

    #[test]
    fn commands_run_through_the_shell() {
        assert_eq!(command_invocation("echo hi", Os::Windows).unwrap(), ["cmd", "/c", "echo hi"]);
        assert_eq!(command_invocation("echo hi", Os::MacOs).unwrap(), ["sh", "-c", "echo hi"]);
        assert_eq!(command_invocation("", Os::Linux), None);
    }

    #[test]
    fn desktop_entries() {
        let firefox = "[Desktop Entry]\nType=Application\nName=Firefox\nName[de]=Feuerfuchs\nExec=firefox %u\n\n[Desktop Action new-window]\nName=New Window\n";
        assert_eq!(parse_desktop_entry(firefox).as_deref(), Some("Firefox"));
        assert_eq!(parse_desktop_entry("[Desktop Entry]\nType=Application\nName=Hidden\nNoDisplay=true\n"), None);
        assert_eq!(parse_desktop_entry("[Desktop Entry]\nType=Link\nName=Docs\n"), None);
    }
}
