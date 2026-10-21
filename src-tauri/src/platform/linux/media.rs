use crate::platform::linux::input::LinuxInput;
use crate::platform::traits::MediaControl;
use std::process::Command;
use std::sync::Arc;

pub struct LinuxMedia {
    input: Arc<LinuxInput>,
}

impl LinuxMedia {
    pub fn new(input: Arc<LinuxInput>) -> Self {
        Self { input }
    }

    fn control_mpris_or_fallback(&self, method: &str, fallback_vk: impl FnOnce()) {
        let script = format!(
            r#"
for p in $(busctl --user list 2>/dev/null | awk '/org\.mpris\.MediaPlayer2/ {{print $1}}'); do
    if busctl --user call "$p" /org/mpris/MediaPlayer2 org.mpris.MediaPlayer2.Player "{}" 2>/dev/null; then
        exit 0
    fi
done
exit 1
"#,
            method
        );
        let success = Command::new("sh")
            .arg("-c")
            .arg(script)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);

        if !success {
            println!("[Linux Media] MPRIS player not reachable, invoking virtual keyboard fallback for {}", method);
            fallback_vk();
        }
    }
}

impl MediaControl for LinuxMedia {
    fn play_pause(&self) {
        let input = self.input.clone();
        self.control_mpris_or_fallback("PlayPause", move || input.emit_play_pause());
    }

    fn next_track(&self) {
        let input = self.input.clone();
        self.control_mpris_or_fallback("Next", move || input.emit_next_track());
    }

    fn prev_track(&self) {
        let input = self.input.clone();
        self.control_mpris_or_fallback("Previous", move || input.emit_prev_track());
    }
}
