/// Matches foreground application details against known profile patterns.
/// Shared across Linux, Windows, and macOS platform implementations.
pub fn match_profile(app: &str, wm_class: &str, title: &str) -> &'static str {
    let query = format!("{} {} {}", wm_class, app, title).to_lowercase();

    // 1. Media & Streaming (checked first so YouTube/Netflix in browser stays media)
    let media_patterns = [
        "youtube", "netflix", "spotify", "vlc", "mpv", "twitch", "prime video",
        "soundcloud", "disney", "jellyfin", "plex", "rhythmbox", "audacious",
        "apple music", "deezer", "tidal", "foobar2000", "aimp", "musicbee",
    ];
    for pat in &media_patterns {
        if query.contains(pat) {
            return "media_video";
        }
    }

    // 2. Code Editors & IDEs
    let code_patterns = [
        "code", "cursor", "zed", "sublime", "nvim", "vim", "pycharm", "intellij",
        "rustrover", "webstorm", "android-studio", "clion", "eclipse", "neovim",
        "visual studio", "devenv", "notepad++", "rider", "goland", "datagrip",
    ];
    for pat in &code_patterns {
        if query.contains(pat) {
            return "code_editor";
        }
    }

    // 3. File Managers
    let file_patterns = [
        "nautilus", "dolphin", "thunar", "pcmanfm", "filee", "explorer", "nemo",
        "caja", "files", "org.gnome.nautilus", "totalcmd", "directoryopus", "7zfm",
    ];
    for pat in &file_patterns {
        if query.contains(pat) {
            return "file_manager";
        }
    }

    // 4. Web Browsers
    let browser_patterns = [
        "firefox", "chrome", "chromium", "zen", "brave", "edge", "msedge", "opera",
        "vivaldi", "epiphany", "google-chrome", "arc", "waterfox", "tor browser",
    ];
    for pat in &browser_patterns {
        if query.contains(pat) {
            return "browser_nav";
        }
    }

    // 5. Creative & Design
    let design_patterns = [
        "figma", "blender", "resolve", "photoshop", "inkscape", "gimp",
        "krita", "illustrator", "premiere", "darktable", "freecad",
        "afterfx", "lightroom", "indesign", "coreldraw", "canva", "autocad",
    ];
    for pat in &design_patterns {
        if query.contains(pat) {
            return "creative_design";
        }
    }

    "default"
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ContextEvent {
    pub profile_id: String,
    pub app: String,
    pub title: String,
    pub wm_class: String,
}

