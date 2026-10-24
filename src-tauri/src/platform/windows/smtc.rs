//! Own System Media Transport Controls (SMTC) session.
//!
//! Bluetooth AVRCP play/pause/next/previous never reach the keyboard path on Windows; they
//! go to the *current* media session. When YouTube (or any browser tab / player) holds that
//! role, the dial click pauses it and Dial Control never sees the click. So we become a media
//! session ourselves: a muted `MediaPlayer` looping silence, whose transport controls receive
//! the button presses. A guard re-asserts the session whenever another app takes over.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use windows::core::HSTRING;
use windows::Foundation::{TypedEventHandler, Uri};
use windows::Media::Control::GlobalSystemMediaTransportControlsSessionManager as SessionManager;
use windows::Media::Core::MediaSource;
use windows::Media::Playback::MediaPlayer;
use windows::Media::{
    MediaPlaybackStatus, MediaPlaybackType, SystemMediaTransportControls,
    SystemMediaTransportControlsButton as Button,
    SystemMediaTransportControlsButtonPressedEventArgs as ButtonArgs,
};

const SESSION_TITLE: &str = "Dial Control";

static OWN_ID_KNOWN: AtomicBool = AtomicBool::new(false);

/// What a media-button press means for the dial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaButton {
    Click,
    Cancel,
    Ignore,
}

/// Play / Pause (AVRCP has no toggle) are the dial click; Next / Previous are the
/// double / triple press gestures that dismiss the HUD.
pub fn classify_button(button: i32) -> MediaButton {
    if button == Button::Play.0 || button == Button::Pause.0 {
        MediaButton::Click
    } else if button == Button::Next.0 || button == Button::Previous.0 {
        MediaButton::Cancel
    } else {
        MediaButton::Ignore
    }
}

fn silence_path() -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join("dial-control");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("silence.wav");
    if !path.exists() {
        // 10s of 8kHz mono 16-bit silence.
        let samples: u32 = 8000 * 10;
        let data_len = samples * 2;
        let mut wav = Vec::with_capacity(44 + data_len as usize);
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&(36 + data_len).to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes()); // PCM
        wav.extend_from_slice(&1u16.to_le_bytes()); // mono
        wav.extend_from_slice(&8000u32.to_le_bytes());
        wav.extend_from_slice(&16000u32.to_le_bytes()); // byte rate
        wav.extend_from_slice(&2u16.to_le_bytes()); // block align
        wav.extend_from_slice(&16u16.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.resize(44 + data_len as usize, 0);
        std::fs::write(&path, wav)?;
    }
    Ok(path)
}

fn file_uri(path: &std::path::Path) -> String {
    format!("file:///{}", path.to_string_lossy().replace('\\', "/").replace(' ', "%20"))
}

fn reassert(player: &MediaPlayer, smtc: &SystemMediaTransportControls) {
    let _ = smtc.SetPlaybackStatus(MediaPlaybackStatus::Paused);
    let _ = player.Pause();
    let _ = player.Play();
    let _ = smtc.SetPlaybackStatus(MediaPlaybackStatus::Playing);
}

fn own_session_id(mgr: &SessionManager) -> Option<String> {
    let sessions = mgr.GetSessions().ok()?;
    for session in sessions {
        if let Ok(op) = session.TryGetMediaPropertiesAsync() {
            if let Ok(props) = op.join() {
                if props.Title().map(|t| t.to_string()).unwrap_or_default() == SESSION_TITLE {
                    return session.SourceAppUserModelId().ok().map(|s| s.to_string());
                }
            }
        }
    }
    None
}

fn current_id(mgr: &SessionManager) -> Option<String> {
    mgr.GetCurrentSession().ok()?.SourceAppUserModelId().ok().map(|s| s.to_string())
}

/// Starts the session on a dedicated MTA thread. `on_button` runs on a WinRT thread-pool
/// thread, so it must return quickly (the dial handlers already hand work to workers).
pub fn start(on_button: impl Fn(MediaButton) + Send + Sync + 'static) {
    thread::spawn(move || unsafe {
        // COINIT_MULTITHREADED: WinRT callbacks are delivered without a message pump.
        let _ = crate::platform::windows::ffi::CoInitializeEx(std::ptr::null_mut(), 0);

        let setup = || -> windows::core::Result<(MediaPlayer, SystemMediaTransportControls)> {
            let player = MediaPlayer::new()?;
            player.CommandManager()?.SetIsEnabled(false)?;
            player.SetVolume(0.0)?;

            let smtc = player.SystemMediaTransportControls()?;
            smtc.SetIsEnabled(true)?;
            smtc.SetIsPlayEnabled(true)?;
            smtc.SetIsPauseEnabled(true)?;
            smtc.SetIsNextEnabled(true)?;
            smtc.SetIsPreviousEnabled(true)?;
            smtc.SetIsStopEnabled(true)?;

            let updater = smtc.DisplayUpdater()?;
            updater.SetType(MediaPlaybackType::Music)?;
            let music = updater.MusicProperties()?;
            music.SetTitle(&HSTRING::from(SESSION_TITLE))?;
            music.SetArtist(&HSTRING::from("dial input"))?;
            updater.Update()?;
            smtc.SetPlaybackStatus(MediaPlaybackStatus::Playing)?;

            smtc.ButtonPressed(&TypedEventHandler::new(
                move |_: windows::core::Ref<SystemMediaTransportControls>,
                      args: windows::core::Ref<ButtonArgs>| {
                    if let Some(args) = args.as_ref() {
                        if let Ok(button) = args.Button() {
                            let kind = classify_button(button.0);
                            println!("[Windows Dial] SMTC button {:?} -> {:?}", button, kind);
                            on_button(kind);
                        }
                    }
                    Ok(())
                },
            ))?;

            let path = silence_path().map_err(|e| windows::core::Error::new(
                windows::core::HRESULT(-2147467259), // E_FAIL
                e.to_string(),
            ))?;
            let uri = Uri::CreateUri(&HSTRING::from(file_uri(&path)))?;
            let source = MediaSource::CreateFromUri(&uri)?;
            player.SetSource(&source)?;
            player.SetIsLoopingEnabled(true)?;
            player.Play()?;
            Ok((player, smtc))
        };

        let (player, smtc) = match setup() {
            Ok(v) => v,
            Err(e) => {
                eprintln!("[Windows Dial] Could not start SMTC media session: {e}");
                return;
            }
        };
        println!("[Windows Dial] SMTC media session active (dial clicks now routed to Dial Control).");

        // Guard: keep the session current when a browser/player grabs media focus.
        let Ok(mgr) = SessionManager::RequestAsync().and_then(|op| op.join()) else {
            eprintln!("[Windows Dial] SMTC session manager unavailable; focus guard disabled.");
            loop {
                thread::sleep(Duration::from_secs(3600));
            }
        };
        thread::sleep(Duration::from_millis(500));
        let mut own_id = own_session_id(&mgr);
        OWN_ID_KNOWN.store(own_id.is_some(), Ordering::SeqCst);

        loop {
            if own_id.is_none() {
                own_id = own_session_id(&mgr);
            }
            if let (Some(own), Some(cur)) = (own_id.as_ref(), current_id(&mgr)) {
                if &cur != own {
                    println!("[Windows Dial] Media focus taken by {cur} -> reasserting");
                    for wait_ms in [150u64, 300, 600, 1200] {
                        reassert(&player, &smtc);
                        thread::sleep(Duration::from_millis(wait_ms));
                        if current_id(&mgr).as_ref() == Some(own) {
                            break;
                        }
                    }
                }
            }
            thread::sleep(Duration::from_millis(1000));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_and_pause_are_the_click() {
        assert_eq!(classify_button(Button::Play.0), MediaButton::Click);
        assert_eq!(classify_button(Button::Pause.0), MediaButton::Click);
    }

    #[test]
    fn next_and_previous_cancel() {
        assert_eq!(classify_button(Button::Next.0), MediaButton::Cancel);
        assert_eq!(classify_button(Button::Previous.0), MediaButton::Cancel);
    }

    #[test]
    fn other_buttons_are_ignored() {
        assert_eq!(classify_button(Button::Stop.0), MediaButton::Ignore);
        assert_eq!(classify_button(-1), MediaButton::Ignore);
    }

    #[test]
    fn file_uri_escapes_windows_paths() {
        let uri = file_uri(std::path::Path::new(r"C:\Users\A B\AppData\Local\Temp\dial-control\silence.wav"));
        assert_eq!(uri, "file:///C:/Users/A%20B/AppData/Local/Temp/dial-control/silence.wav");
    }
}
