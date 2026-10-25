//! Short audible cues played through the default output (the earbuds), so the user hears a
//! mode change without looking at the screen. The tone is generated here as a WAV file and
//! handed to each OS's own player, which keeps the app free of audio dependencies.

use std::path::PathBuf;

const SAMPLE_RATE: u32 = 44_100;
const NOTE_MS: u32 = 70;
const FADE_MS: u32 = 6;
const AMPLITUDE: f32 = 0.22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue {
    /// Rising two-note chirp.
    ScrollOn,
    /// Falling two-note chirp.
    ScrollOff,
}

impl Cue {
    fn notes(self) -> [f32; 2] {
        match self {
            Cue::ScrollOn => [660.0, 990.0],
            Cue::ScrollOff => [990.0, 660.0],
        }
    }

    fn file_name(self) -> &'static str {
        match self {
            Cue::ScrollOn => "scroll-on.wav",
            Cue::ScrollOff => "scroll-off.wav",
        }
    }
}

/// 16-bit mono PCM WAV of the cue's notes, each faded in and out to avoid clicks.
pub fn cue_wav(cue: Cue) -> Vec<u8> {
    let per_note = SAMPLE_RATE * NOTE_MS / 1000;
    let fade = (SAMPLE_RATE * FADE_MS / 1000) as f32;
    let mut samples: Vec<i16> = Vec::with_capacity((per_note * 2) as usize);
    for freq in cue.notes() {
        for i in 0..per_note {
            let t = i as f32 / SAMPLE_RATE as f32;
            let envelope = (i as f32 / fade).min((per_note - i) as f32 / fade).min(1.0);
            let v = (t * freq * std::f32::consts::TAU).sin() * AMPLITUDE * envelope;
            samples.push((v * i16::MAX as f32) as i16);
        }
    }

    let data_len = (samples.len() * 2) as u32;
    let mut wav = Vec::with_capacity(44 + data_len as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_len).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
    wav.extend_from_slice(&1u16.to_le_bytes()); // mono
    wav.extend_from_slice(&SAMPLE_RATE.to_le_bytes());
    wav.extend_from_slice(&(SAMPLE_RATE * 2).to_le_bytes());
    wav.extend_from_slice(&2u16.to_le_bytes());
    wav.extend_from_slice(&16u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        wav.extend_from_slice(&s.to_le_bytes());
    }
    wav
}

fn cue_path(cue: Cue) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join("dial-control");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(cue.file_name());
    if !path.exists() {
        std::fs::write(&path, cue_wav(cue))?;
    }
    Ok(path)
}

/// Plays the cue without blocking the caller. Failures are only logged.
pub fn play(cue: Cue) {
    std::thread::spawn(move || {
        let path = match cue_path(cue) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[Cue] Could not write {:?}: {:?}", cue, e);
                return;
            }
        };
        if !play_file(&path) {
            crate::platform::diag::log(&format!("cue {cue:?}: no player available"));
        }
    });
}

#[cfg(target_os = "linux")]
fn play_file(path: &std::path::Path) -> bool {
    // PipeWire, then PulseAudio, then plain ALSA.
    for (cmd, args) in [("pw-play", &[][..]), ("paplay", &[][..]), ("aplay", &["-q"][..])] {
        let status = std::process::Command::new(cmd).args(args).arg(path).status();
        if matches!(status, Ok(s) if s.success()) {
            return true;
        }
    }
    false
}

#[cfg(target_os = "macos")]
fn play_file(path: &std::path::Path) -> bool {
    matches!(std::process::Command::new("afplay").arg(path).status(), Ok(s) if s.success())
}

#[cfg(target_os = "windows")]
fn play_file(path: &std::path::Path) -> bool {
    use crate::platform::windows::ffi::{PlaySoundW, SND_FILENAME, SND_NODEFAULT};
    use std::os::windows::ffi::OsStrExt;
    let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    // Synchronous on this worker thread: SND_ASYNC would return before the file is read.
    unsafe { PlaySoundW(wide.as_ptr(), std::ptr::null_mut(), SND_FILENAME | SND_NODEFAULT) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wav_header_matches_payload() {
        let wav = cue_wav(Cue::ScrollOn);
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..16], b"WAVEfmt ");
        let data_len = u32::from_le_bytes(wav[40..44].try_into().unwrap()) as usize;
        assert_eq!(wav.len(), 44 + data_len);
        let riff_len = u32::from_le_bytes(wav[4..8].try_into().unwrap()) as usize;
        assert_eq!(riff_len, wav.len() - 8);
        // Two notes of NOTE_MS each.
        assert_eq!(data_len, (SAMPLE_RATE * NOTE_MS / 1000 * 2 * 2) as usize);
    }

    #[test]
    fn notes_start_and_end_silent() {
        let wav = cue_wav(Cue::ScrollOff);
        let first = i16::from_le_bytes([wav[44], wav[45]]);
        let last = i16::from_le_bytes([wav[wav.len() - 2], wav[wav.len() - 1]]);
        assert_eq!(first, 0);
        assert!(last.abs() < 200, "last sample {last} should be faded out");
    }

    #[test]
    fn on_rises_and_off_falls() {
        assert!(Cue::ScrollOn.notes()[0] < Cue::ScrollOn.notes()[1]);
        assert!(Cue::ScrollOff.notes()[0] > Cue::ScrollOff.notes()[1]);
    }
}
