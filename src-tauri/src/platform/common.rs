const MEDIA_PATTERNS: &[&str] = &[
    "youtube", "netflix", "spotify", "vlc", "mpv", "twitch", "prime video",
    "soundcloud", "disney", "jellyfin", "plex", "rhythmbox", "audacious",
    "apple music", "deezer", "tidal", "foobar2000", "aimp", "musicbee",
];

const CODE_PATTERNS: &[&str] = &[
    "code", "vscode", "vscodium", "cursor", "zed", "sublime", "nvim", "vim", "gvim", "pycharm",
    "intellij", "idea64", "jetbrains", "rustrover", "webstorm", "android studio", "clion", "eclipse", "neovim",
    "visual studio", "devenv", "notepad++", "rider", "goland", "datagrip", "xcode",
];

const FILE_PATTERNS: &[&str] = &[
    "nautilus", "dolphin", "thunar", "pcmanfm", "filee", "explorer", "nemo",
    "caja", "files", "finder", "totalcmd", "directoryopus", "dopus", "7zfm",
];

const BROWSER_PATTERNS: &[&str] = &[
    "firefox", "chrome", "chromium", "zen", "brave", "edge", "msedge", "opera",
    "vivaldi", "epiphany", "arc", "waterfox", "tor browser", "safari", "librewolf", "floorp",
];

const DESIGN_PATTERNS: &[&str] = &[
    "figma", "blender", "resolve", "photoshop", "inkscape", "gimp",
    "krita", "illustrator", "premiere", "darktable", "freecad",
    "afterfx", "lightroom", "indesign", "coreldraw", "canva", "autocad",
];

/// A word from an app id / title, plus the looser forms it may be matched by:
/// `notepad++` also matches `notepad++`, `pycharm64.exe` also matches `pycharm`.
struct Token {
    raw: String,
    base: String,
}

fn tokenize(s: &str) -> Vec<Token> {
    s.to_lowercase()
        .split(|c: char| !(c.is_alphanumeric() || c == '+'))
        .filter(|t| !t.is_empty())
        .map(|raw| {
            let no_plus = raw.trim_end_matches('+');
            let no_digits = no_plus.trim_end_matches(|c: char| c.is_ascii_digit());
            let base = if no_digits.is_empty() { no_plus } else { no_digits };
            Token {
                raw: raw.to_string(),
                base: base.to_string(),
            }
        })
        .collect()
}

fn token_matches(token: &Token, word: &str) -> bool {
    token.raw == word || token.base == word || token.raw.trim_end_matches('+') == word
}

/// Whole-word (or whole-phrase) match, so `zed` never matches "optimized"
/// and `code` never matches "unicode".
fn matches_any(tokens: &[Token], patterns: &[&str]) -> bool {
    patterns.iter().any(|pat| {
        let words: Vec<String> = pat
            .to_lowercase()
            .split(|c: char| !(c.is_alphanumeric() || c == '+'))
            .filter(|w| !w.is_empty())
            .map(str::to_string)
            .collect();
        if words.is_empty() || words.len() > tokens.len() {
            return false;
        }
        tokens
            .windows(words.len())
            .any(|win| win.iter().zip(&words).all(|(t, w)| token_matches(t, w)))
    })
}

/// Matches foreground application details against known profile patterns.
/// Shared across Linux, Windows, and macOS platform implementations.
///
/// The application identity (WM_CLASS / exe name / bundle id) is trusted over the
/// window title, so a browser tab titled "Upload files" stays a browser and a VS Code
/// tab named `spotify.ts` stays an editor. The one deliberate exception is streaming
/// media, where the title is the only signal (YouTube inside Chrome).
pub fn match_profile(app: &str, wm_class: &str, title: &str) -> &'static str {
    let identity = tokenize(&format!("{} {}", wm_class, app));
    let title_tokens = tokenize(title);

    if matches_any(&identity, MEDIA_PATTERNS) {
        return "media_video";
    }
    let identity_categories: [(&[&str], &'static str); 3] = [
        (CODE_PATTERNS, "code_editor"),
        (FILE_PATTERNS, "file_manager"),
        (DESIGN_PATTERNS, "creative_design"),
    ];
    for (patterns, id) in identity_categories {
        if matches_any(&identity, patterns) {
            return id;
        }
    }
    if matches_any(&title_tokens, MEDIA_PATTERNS) {
        return "media_video";
    }
    if matches_any(&identity, BROWSER_PATTERNS) {
        return "browser_nav";
    }

    let title_categories: [(&[&str], &'static str); 4] = [
        (CODE_PATTERNS, "code_editor"),
        (FILE_PATTERNS, "file_manager"),
        (BROWSER_PATTERNS, "browser_nav"),
        (DESIGN_PATTERNS, "creative_design"),
    ];
    // Only fall back to the title when the identity is unknown or a generic host
    // (terminal running nvim, Electron shell, etc).
    for (patterns, id) in title_categories {
        if matches_any(&title_tokens, patterns) {
            return id;
        }
    }

    "default"
}

/// Window class to use for profile matching. Every Electron/CEF app on Windows
/// (Discord, Slack, Teams, Obsidian, VS Code, Spotify) shares the class
/// `Chrome_WidgetWin_*`, which would otherwise classify all of them as a browser.
pub fn class_for_matching<'a>(app: &str, class_name: &'a str) -> &'a str {
    if !app.is_empty() && class_name.starts_with("Chrome_WidgetWin") {
        ""
    } else {
        class_name
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextEvent {
    pub profile_id: String,
    pub app: String,
    pub title: String,
    pub wm_class: String,
}

#[cfg(test)]
mod tests {
    use super::match_profile;

    #[test]
    fn linux_wm_classes() {
        assert_eq!(match_profile("code", "code", "main.rs - proj - Visual Studio Code"), "code_editor");
        assert_eq!(match_profile("org.gnome.Nautilus", "org.gnome.Nautilus", "Downloads"), "file_manager");
        assert_eq!(match_profile("zen", "zen", "GitHub"), "browser_nav");
        assert_eq!(match_profile("firefox_firefox", "firefox_firefox", "Mozilla Firefox"), "browser_nav");
        assert_eq!(match_profile("google-chrome", "google-chrome", "New Tab"), "browser_nav");
        assert_eq!(match_profile("Spotify", "Spotify", "Spotify Premium"), "media_video");
        assert_eq!(match_profile("org.inkscape.Inkscape", "org.inkscape.Inkscape", "drawing.svg"), "creative_design");
        assert_eq!(match_profile("gimp-2.10", "gimp-2.10", "image.png"), "creative_design");
        assert_eq!(match_profile("sublime_text", "sublime_text", "notes"), "code_editor");
        assert_eq!(match_profile("dev.zed.Zed", "dev.zed.Zed", "proj"), "code_editor");
    }

    #[test]
    fn windows_exe_names() {
        assert_eq!(match_profile("code", "Chrome_WidgetWin_1", "x.ts - Visual Studio Code"), "code_editor");
        assert_eq!(match_profile("pycharm64", "SunAwtFrame", "proj – main.py"), "code_editor");
        assert_eq!(match_profile("idea64", "SunAwtFrame", "proj – Main.java"), "code_editor");
        assert_eq!(match_profile("notepad++", "Notepad++", "new 1"), "code_editor");
        assert_eq!(match_profile("explorer", "CabinetWClass", "Downloads"), "file_manager");
        assert_eq!(match_profile("totalcmd64", "TTOTAL_CMD", "Total Commander"), "file_manager");
        assert_eq!(match_profile("msedge", "Chrome_WidgetWin_1", "New tab - Profile 1 - Microsoft Edge"), "browser_nav");
        assert_eq!(match_profile("desktop", "Progman", "Program Manager"), "default");
        // Plain Notepad must not be mistaken for Notepad++.
        assert_eq!(match_profile("notepad", "Notepad", "todo.txt - Notepad"), "default");
    }

    #[test]
    fn macos_bundle_ids() {
        assert_eq!(match_profile("Code", "com.microsoft.VSCode", "main.rs"), "code_editor");
        assert_eq!(match_profile("Finder", "com.apple.finder", "Documents"), "file_manager");
        assert_eq!(match_profile("Safari", "com.apple.Safari", "Apple"), "browser_nav");
        assert_eq!(match_profile("Arc", "company.thebrowser.Browser", "Space"), "browser_nav");
        assert_eq!(match_profile("Music", "com.apple.Music", "Song"), "media_video");
        assert_eq!(match_profile("Xcode", "com.apple.dt.Xcode", "App.swift"), "code_editor");
        assert_eq!(match_profile("Cursor", "com.todesktop.230313mzl4w4u92", "proj"), "code_editor");
    }

    #[test]
    fn media_in_browser_is_media() {
        assert_eq!(match_profile("firefox", "firefox", "Lofi beats - YouTube — Mozilla Firefox"), "media_video");
        assert_eq!(match_profile("chrome", "Chrome_WidgetWin_1", "Netflix - Google Chrome"), "media_video");
        assert_eq!(match_profile("zen", "zen", "Watching on Prime Video"), "media_video");
    }

    #[test]
    fn substring_false_positives_are_gone() {
        // Each of these used to switch profile purely because of a substring in the title.
        assert_eq!(match_profile("gnome-text-editor", "gnome-text-editor", "unicode table.txt"), "default");
        assert_eq!(match_profile("libreoffice-writer", "libreoffice-writer", "Optimized report"), "default");
        assert_eq!(match_profile("libreoffice-writer", "libreoffice-writer", "Knowledge base"), "default");
        assert_eq!(match_profile("org.gnome.Settings", "org.gnome.Settings", "Search"), "default");
        assert_eq!(match_profile("SearchHost", "Windows.UI.Core.CoreWindow", "Search"), "default");
    }

    #[test]
    fn identity_beats_title() {
        assert_eq!(match_profile("chrome", "google-chrome", "Upload files - Google Chrome"), "browser_nav");
        assert_eq!(match_profile("code", "code", "spotify.ts - player - Visual Studio Code"), "code_editor");
        assert_eq!(match_profile("org.gnome.Nautilus", "org.gnome.Nautilus", "Spotify"), "file_manager");
    }

    #[test]
    fn title_fallback_for_generic_hosts() {
        assert_eq!(match_profile("org.gnome.Ptyxis", "org.gnome.Ptyxis", "nvim main.rs"), "code_editor");
        assert_eq!(match_profile("kitty", "kitty", "vim notes.md"), "code_editor");
        assert_eq!(match_profile("", "", "Figma Desktop"), "creative_design");
    }

    #[test]
    fn electron_apps_are_not_browsers_on_windows() {
        use super::class_for_matching;
        let class = "Chrome_WidgetWin_1";
        assert_eq!(match_profile("discord", class_for_matching("discord", class), "Discord"), "default");
        assert_eq!(match_profile("slack", class_for_matching("slack", class), "Slack"), "default");
        assert_eq!(match_profile("spotify", class_for_matching("spotify", class), "Spotify"), "media_video");
        assert_eq!(match_profile("chrome", class_for_matching("chrome", class), "New Tab"), "browser_nav");
        // Unreadable process path: keep the class as the only remaining hint.
        assert_eq!(class_for_matching("", class), class);
        assert_eq!(match_profile("", class_for_matching("", class), ""), "browser_nav");
    }

    #[test]
    fn jetbrains_on_linux_and_no_idea_false_positive() {
        assert_eq!(match_profile("jetbrains-idea", "jetbrains-idea", "proj – Main.java"), "code_editor");
        assert_eq!(match_profile("org.gnome.TextEditor", "org.gnome.TextEditor", "my idea.txt"), "default");
    }

    #[test]
    fn empty_and_unknown_inputs() {
        assert_eq!(match_profile("", "", ""), "default");
        assert_eq!(match_profile("obsidian", "obsidian", "Vault"), "default");
        assert_eq!(match_profile("????", "", "🎵🎵"), "default");
    }
}
