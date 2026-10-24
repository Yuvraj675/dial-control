//! Platform-agnostic dial decision logic.
//!
//! Every OS backend (Win32 hooks, CoreGraphics taps, BlueZ D-Bus) funnels raw hardware
//! signals through these pure functions. Keeping them free of FFI lets the exact same
//! behaviour be unit-tested on any host, including the Windows and macOS paths.

/// Locked volume is clamped into this band so a rotary turn always has headroom to
/// register in both directions (a CW turn at 100% or a CCW turn at 0% produces no event).
pub const VOLUME_HEADROOM_MIN: i32 = 6;
pub const VOLUME_HEADROOM_MAX: i32 = 94;

/// Volume deltas at or below this many percent are treated as rounding echoes
/// (AVRCP 0..127 <-> 0..100% round trips), never as a physical dial detent.
pub const VOLUME_ECHO_TOLERANCE: i32 = 1;

/// Shared click debounce for backends that can observe one physical press twice
/// (e.g. Windows Raw Input + low-level keyboard hook).
pub const CLICK_DEBOUNCE_MS: u64 = 400;

/// Rotation notifications closer together than this are one hardware burst.
pub const ROTATION_BURST_MS: u64 = 40;

/// Within this window after Dial Control set the brightness, hardware reads are stale.
pub const BRIGHTNESS_SETTLE_MS: u64 = 2500;

pub const BRIGHTNESS_STEP: i32 = 5;

pub fn headroom_lock(current_pct: i32) -> i32 {
    current_pct.clamp(VOLUME_HEADROOM_MIN, VOLUME_HEADROOM_MAX)
}

/// CW rotation scrolls down (-1), CCW scrolls up (+1), matching a physical mouse wheel.
pub fn scroll_delta_for(direction: i32) -> i32 {
    if direction > 0 {
        -1
    } else {
        1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClickAction {
    /// Contact bounce or a duplicate observation of the same press.
    Debounced,
    ExitScroll,
    OpenHud,
    Confirm,
}

pub fn classify_click(
    now_ms: u64,
    last_click_ms: u64,
    debounce_ms: u64,
    is_hud_open: bool,
    is_scroll_mode: bool,
) -> ClickAction {
    if last_click_ms != 0 && now_ms.saturating_sub(last_click_ms) < debounce_ms {
        ClickAction::Debounced
    } else if is_scroll_mode {
        ClickAction::ExitScroll
    } else if !is_hud_open {
        ClickAction::OpenHud
    } else {
        ClickAction::Confirm
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VolumeAction {
    /// Nothing to do (HUD closed, echo of our own restore, or rounding noise).
    Ignore,
    /// Dial detent while the volume is locked: snap back to `locked_pct` and
    /// emit a rotation (or scroll tick) in `direction`.
    Turn { direction: i32 },
    /// HUD is open in volume mode (lock released): the change is real, only
    /// mirror it on the gauge.
    NaturalChange { pct: i32 },
}

/// Decides what an endpoint volume notification means. Used by backends that observe
/// rotation as an absolute volume value (Windows Core Audio, macOS CoreAudio).
pub fn classify_volume_change(
    new_pct: i32,
    locked_pct: i32,
    is_hud_open: bool,
    is_scroll_mode: bool,
) -> VolumeAction {
    if !is_hud_open && !is_scroll_mode {
        return VolumeAction::Ignore;
    }
    if locked_pct < 0 {
        if is_hud_open && !is_scroll_mode {
            return VolumeAction::NaturalChange {
                pct: new_pct.clamp(0, 100),
            };
        }
        return VolumeAction::Ignore;
    }
    let delta = new_pct - locked_pct;
    if delta.abs() <= VOLUME_ECHO_TOLERANCE {
        return VolumeAction::Ignore;
    }
    VolumeAction::Turn {
        direction: delta.signum(),
    }
}

/// Windows floating-headroom model: the endpoint volume floats while navigating and is
/// recentred to 50% near the edges. Windows re-sends our own set ~1ms later with GUID_NULL.
pub const FLOAT_ECHO_TOLERANCE: f32 = 0.015;
pub const FLOAT_ECHO_WINDOW_MS: u64 = 800;
/// ~1/3 of a 16-step AVRCP grid step (0.0625).
pub const FLOAT_JITTER: f32 = 0.02;
/// Volume mode mirrors every real change, including 2% keyboard steps.
pub const FLOAT_JITTER_VOLUME_MODE: f32 = 0.005;
pub const FLOAT_RECENTER_LOW: f32 = 0.15;
pub const FLOAT_RECENTER_HIGH: f32 = 0.85;
pub const FLOAT_RECENTER_LEVEL: f32 = 0.50;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FloatingAction {
    /// Not a dial turn (echo of our own set, or HUD closed): just track the level.
    Track,
    /// Sub-step noise: ignore without moving the reference level.
    Jitter,
    Turn { direction: i32, recenter: bool },
}

/// `echo_armed` is true only between one of our own volume sets and the next real turn;
/// a real turn disarms it so a quick back-and-forth onto the set level is not mistaken
/// for an echo.
pub fn classify_floating_change(
    new_level: f32,
    prev_level: f32,
    last_set_level: f32,
    ms_since_set: u64,
    echo_armed: bool,
    is_hud_open: bool,
    is_scroll_mode: bool,
    is_volume_mode: bool,
) -> FloatingAction {
    if echo_armed
        && (new_level - last_set_level).abs() < FLOAT_ECHO_TOLERANCE
        && ms_since_set < FLOAT_ECHO_WINDOW_MS
    {
        return FloatingAction::Track;
    }
    if !is_hud_open && !is_scroll_mode {
        return FloatingAction::Track;
    }
    let d = new_level - prev_level;
    let threshold = if is_volume_mode { FLOAT_JITTER_VOLUME_MODE } else { FLOAT_JITTER };
    if d.abs() < threshold {
        return FloatingAction::Jitter;
    }
    FloatingAction::Turn {
        direction: if d > 0.0 { 1 } else { -1 },
        // Never in volume mode, where recentring would make >85% unreachable.
        recenter: !is_volume_mode && (new_level <= FLOAT_RECENTER_LOW || new_level >= FLOAT_RECENTER_HIGH),
    }
}

/// True when a rotation notification arriving at `now_ms` belongs to the same
/// hardware burst as the previous one and must not produce a second step.
pub fn is_rotation_burst(now_ms: u64, last_rotation_ms: u64) -> bool {
    last_rotation_ms != 0 && now_ms.saturating_sub(last_rotation_ms) < ROTATION_BURST_MS
}

/// Decoded macOS `NX_SUBTYPE_AUX_CONTROL_BUTTONS` payload (`NSEvent.data1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuxKey {
    pub key_type: u32,
    pub is_down: bool,
    pub is_repeat: bool,
}

/// Layout of data1: `[key_type:16][key_state:8][flags:8]` where key_state is
/// 0x0A for key-down and 0x0B for key-up, and bit 0 of flags marks auto-repeat.
pub fn decode_aux_key(data1: i64) -> AuxKey {
    let key_type = ((data1 >> 16) & 0xFFFF) as u32;
    let key_state = (data1 >> 8) & 0xFF;
    AuxKey {
        key_type,
        is_down: key_state == 0x0A,
        is_repeat: (data1 & 0x01) != 0,
    }
}

/// HID Consumer page (0x0C) usages emitted by a Bluetooth AVRCP "click".
/// AVRCP has no toggle opcode, so earbuds send discrete Play / Pause, which Windows
/// delivers as WM_APPCOMMAND only (no virtual key reaches WH_KEYBOARD_LL).
pub const HID_USAGE_PLAY: u16 = 0xB0;
pub const HID_USAGE_PAUSE: u16 = 0xB1;
pub const HID_USAGE_PLAY_PAUSE: u16 = 0xCD;

pub fn is_click_usage(usage: u16) -> bool {
    matches!(usage, HID_USAGE_PLAY | HID_USAGE_PAUSE | HID_USAGE_PLAY_PAUSE)
}

/// Consumer-control reports are level-triggered (usage present while held).
/// Returns `(pressed_edge, now_down)` for the click button given the previous state.
pub fn consumer_click_edge(was_down: bool, usages: &[u16]) -> (bool, bool) {
    let now_down = usages.iter().any(|&u| is_click_usage(u));
    (now_down && !was_down, now_down)
}

pub fn brightness_step_target(current_pct: i32, direction: i32) -> i32 {
    let step = if direction > 0 {
        BRIGHTNESS_STEP
    } else {
        -BRIGHTNESS_STEP
    };
    (current_pct + step).clamp(0, 100)
}

/// Hardware brightness reads lag behind our own writes (WMI applies asynchronously),
/// so recently-set values must be stepped from the cache instead of re-queried.
pub fn brightness_cache_is_fresh(now_ms: u64, last_set_ms: u64) -> bool {
    last_set_ms != 0 && now_ms.saturating_sub(last_set_ms) < BRIGHTNESS_SETTLE_MS
}

/// Media remote command delivered to our own Now Playing session (macOS `MPRemoteCommandCenter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteCommand {
    Play,
    Pause,
    Toggle,
    Next,
    Previous,
}

/// Play / Pause / Toggle are the dial click; Next / Previous are the double / triple press
/// gestures that dismiss the HUD.
pub fn remote_command_is_click(cmd: RemoteCommand) -> bool {
    matches!(cmd, RemoteCommand::Play | RemoteCommand::Pause | RemoteCommand::Toggle)
}

/// Playback state to report after `cmd`. AVRCP has no toggle: the earbuds alternate PLAY and
/// PAUSE from their own idea of the state, and the system drops a PLAY while we already report
/// playing, so each click must leave us reporting what the next (opposite) press expects.
pub fn reported_playing_after(cmd: RemoteCommand, playing: bool) -> bool {
    match cmd {
        RemoteCommand::Play => true,
        RemoteCommand::Pause => false,
        RemoteCommand::Toggle => !playing,
        RemoteCommand::Next | RemoteCommand::Previous => playing,
    }
}

pub fn now_millis() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headroom_lock_clamps_extremes() {
        assert_eq!(headroom_lock(100), 94);
        assert_eq!(headroom_lock(0), 6);
        assert_eq!(headroom_lock(-5), 6);
        assert_eq!(headroom_lock(50), 50);
        assert_eq!(headroom_lock(94), 94);
        assert_eq!(headroom_lock(6), 6);
    }

    #[test]
    fn scroll_direction_matches_mouse_wheel() {
        assert_eq!(scroll_delta_for(1), -1);
        assert_eq!(scroll_delta_for(-1), 1);
        assert_eq!(scroll_delta_for(0), 1);
    }

    #[test]
    fn first_click_ever_is_never_debounced() {
        // LAST_CLICK starts at 0; a clock reading near 0 (fresh VM, tests) must still open.
        assert_eq!(classify_click(10, 0, 400, false, false), ClickAction::OpenHud);
    }

    #[test]
    fn click_state_machine() {
        assert_eq!(classify_click(10_000, 0, 400, false, false), ClickAction::OpenHud);
        assert_eq!(classify_click(10_000, 5_000, 400, true, false), ClickAction::Confirm);
        assert_eq!(classify_click(10_000, 5_000, 400, false, true), ClickAction::ExitScroll);
        // Scroll mode wins even if the HUD flag is still set.
        assert_eq!(classify_click(10_000, 5_000, 400, true, true), ClickAction::ExitScroll);
    }

    #[test]
    fn duplicate_click_observations_are_debounced() {
        // Raw Input and the keyboard hook both see one press 15ms apart.
        assert_eq!(classify_click(10_015, 10_000, 400, true, false), ClickAction::Debounced);
        assert_eq!(classify_click(10_399, 10_000, 400, false, false), ClickAction::Debounced);
        assert_eq!(classify_click(10_400, 10_000, 400, false, false), ClickAction::OpenHud);
    }

    #[test]
    fn clock_going_backwards_does_not_panic_or_lock_out() {
        // saturating_sub -> 0 elapsed -> debounced, but never a panic.
        assert_eq!(classify_click(5_000, 10_000, 400, false, false), ClickAction::Debounced);
    }

    #[test]
    fn volume_ignored_when_hud_closed() {
        assert_eq!(classify_volume_change(60, 50, false, false), VolumeAction::Ignore);
        assert_eq!(classify_volume_change(60, -1, false, false), VolumeAction::Ignore);
    }

    #[test]
    fn volume_turn_directions() {
        assert_eq!(
            classify_volume_change(56, 50, true, false),
            VolumeAction::Turn { direction: 1 }
        );
        assert_eq!(
            classify_volume_change(44, 50, true, false),
            VolumeAction::Turn { direction: -1 }
        );
        assert_eq!(
            classify_volume_change(56, 50, false, true),
            VolumeAction::Turn { direction: 1 }
        );
    }

    #[test]
    fn volume_echo_and_rounding_noise_is_swallowed() {
        // Our own restore lands exactly on the lock.
        assert_eq!(classify_volume_change(50, 50, true, false), VolumeAction::Ignore);
        // AVRCP 0..127 quantisation echoes back off by one percent.
        assert_eq!(classify_volume_change(51, 50, true, false), VolumeAction::Ignore);
        assert_eq!(classify_volume_change(49, 50, true, false), VolumeAction::Ignore);
        assert_eq!(
            classify_volume_change(52, 50, true, false),
            VolumeAction::Turn { direction: 1 }
        );
    }

    #[test]
    fn volume_mode_mirrors_natural_changes() {
        assert_eq!(
            classify_volume_change(73, -1, true, false),
            VolumeAction::NaturalChange { pct: 73 }
        );
        assert_eq!(
            classify_volume_change(140, -1, true, false),
            VolumeAction::NaturalChange { pct: 100 }
        );
        // Scroll mode without a lock must never leak into the gauge.
        assert_eq!(classify_volume_change(73, -1, false, true), VolumeAction::Ignore);
    }

    #[test]
    fn locked_extremes_always_have_headroom() {
        // The whole point of the clamp: from either edge both turn directions register.
        for start in [0, 100] {
            let lock = headroom_lock(start);
            let cw = (lock + 6).min(100);
            let ccw = (lock - 6).max(0);
            assert_eq!(classify_volume_change(cw, lock, true, false), VolumeAction::Turn { direction: 1 });
            assert_eq!(classify_volume_change(ccw, lock, true, false), VolumeAction::Turn { direction: -1 });
        }
    }

    const STEP: f32 = 1.0 / 16.0;

    /// Drives the Windows floating model exactly like `callback_on_notify` does.
    struct FloatingSim {
        level: f32,
        last_audio: f32,
        last_set: f32,
        ms_since_set: u64,
        armed: bool,
        volume_mode: bool,
        directions: Vec<i32>,
    }

    impl FloatingSim {
        fn open_at(level: f32) -> Self {
            // init_hud_rotation: recentre if outside 25..75%, and arm the echo filter.
            let start = if !(0.25..=0.75).contains(&level) { 0.5 } else { level };
            FloatingSim { level: start, last_audio: start, last_set: start, ms_since_set: 0, armed: true, volume_mode: false, directions: vec![] }
        }

        fn notify(&mut self, new_level: f32, elapsed_ms: u64) {
            self.ms_since_set += elapsed_ms;
            match classify_floating_change(
                new_level, self.last_audio, self.last_set, self.ms_since_set, self.armed, true, false, self.volume_mode,
            ) {
                FloatingAction::Track => self.last_audio = new_level,
                FloatingAction::Jitter => {}
                FloatingAction::Turn { direction, recenter } => {
                    self.directions.push(direction);
                    self.last_audio = new_level;
                    self.level = new_level;
                    self.armed = false;
                    if recenter {
                        self.level = FLOAT_RECENTER_LEVEL;
                        self.last_audio = FLOAT_RECENTER_LEVEL;
                        self.last_set = FLOAT_RECENTER_LEVEL;
                        self.ms_since_set = 0;
                        self.armed = true;
                        // Windows' GUID_NULL duplicate of our own recentre, 1ms later.
                        self.notify(FLOAT_RECENTER_LEVEL, 1);
                    }
                }
            }
        }

        fn detent(&mut self, dir: i32, elapsed_ms: u64) {
            let next = (self.level + dir as f32 * STEP).clamp(0.0, 1.0);
            self.notify(next, elapsed_ms);
        }
    }

    #[test]
    fn floating_long_cw_spin_never_loses_or_reverses_a_detent() {
        let mut sim = FloatingSim::open_at(0.40);
        for _ in 0..30 {
            sim.detent(1, 120);
        }
        assert_eq!(sim.directions, vec![1; 30]);
        assert!(sim.level < FLOAT_RECENTER_HIGH + STEP);
    }

    #[test]
    fn floating_open_at_full_volume_still_rotates_both_ways() {
        let mut sim = FloatingSim::open_at(1.0);
        sim.detent(1, 200);
        sim.detent(-1, 200);
        sim.detent(-1, 200);
        assert_eq!(sim.directions, vec![1, -1, -1]);
    }

    #[test]
    fn floating_overshoot_back_to_start_is_not_an_echo() {
        // Regression: right after opening (echo window armed at the start level), turning
        // CW then immediately back CCW onto the start level was swallowed as an echo.
        let mut sim = FloatingSim::open_at(0.40);
        sim.detent(1, 150);
        sim.detent(-1, 150);
        assert_eq!(sim.directions, vec![1, -1]);
    }

    #[test]
    fn floating_back_and_forth_after_recenter() {
        let mut sim = FloatingSim::open_at(0.75);
        sim.detent(1, 100); // 0.8125
        sim.detent(1, 100); // 0.875 -> recentre to 0.5
        sim.detent(1, 100); // 0.5625
        sim.detent(-1, 100); // back onto 0.5, within 800ms of the recentre
        assert_eq!(sim.directions, vec![1, 1, 1, -1]);
    }

    #[test]
    fn floating_echo_of_our_own_set_is_ignored() {
        assert_eq!(
            classify_floating_change(0.505, 0.30, 0.50, 1, true, true, false, false),
            FloatingAction::Track
        );
        // Same level long after the set is a real turn.
        assert_eq!(
            classify_floating_change(0.505, 0.30, 0.50, 900, true, true, false, false),
            FloatingAction::Turn { direction: 1, recenter: false }
        );
    }

    #[test]
    fn floating_ignored_when_hud_closed_and_jitter_filtered() {
        assert_eq!(classify_floating_change(0.7, 0.5, 0.0, 10_000, false, false, false, false), FloatingAction::Track);
        assert_eq!(classify_floating_change(0.51, 0.5, 0.0, 10_000, false, true, false, false), FloatingAction::Jitter);
    }

    #[test]
    fn floating_volume_mode_reaches_the_top_and_mirrors_keyboard_steps() {
        // Regression: recentring also ran in volume mode, so >85% was unreachable.
        assert_eq!(
            classify_floating_change(0.90, 0.84, 0.0, 10_000, false, true, false, true),
            FloatingAction::Turn { direction: 1, recenter: false }
        );
        // Windows keyboard volume keys move 2%, which sat on the menu-mode jitter threshold.
        assert_eq!(
            classify_floating_change(0.62, 0.60, 0.0, 10_000, false, true, false, true),
            FloatingAction::Turn { direction: 1, recenter: false }
        );
    }

    #[test]
    fn rotation_burst_window() {
        assert!(!is_rotation_burst(1_000, 0));
        assert!(is_rotation_burst(1_039, 1_000));
        assert!(!is_rotation_burst(1_040, 1_000));
    }

    #[test]
    fn aux_key_down_and_up_are_distinguished() {
        // NX_KEYTYPE_SOUND_UP (0) key-down / key-up exactly as macOS encodes them.
        let down = decode_aux_key((0 << 16) | (0x0A << 8));
        let up = decode_aux_key((0 << 16) | (0x0B << 8));
        assert_eq!(down, AuxKey { key_type: 0, is_down: true, is_repeat: false });
        // Regression: key-up (0x0B) has bit 0 set and used to be read as a key-down,
        // stepping the HUD twice per detent.
        assert_eq!(up, AuxKey { key_type: 0, is_down: false, is_repeat: false });
    }

    #[test]
    fn aux_key_play_and_repeat() {
        let play_down = decode_aux_key((16 << 16) | (0x0A << 8));
        assert_eq!(play_down.key_type, 16);
        assert!(play_down.is_down);
        let held = decode_aux_key((1 << 16) | (0x0A << 8) | 0x01);
        assert_eq!(held.key_type, 1);
        assert!(held.is_down);
        assert!(held.is_repeat);
    }

    #[test]
    fn consumer_click_usages() {
        assert!(is_click_usage(0xB0));
        assert!(is_click_usage(0xB1));
        assert!(is_click_usage(0xCD));
        assert!(!is_click_usage(0xE9)); // Volume Increment
        assert!(!is_click_usage(0xEA)); // Volume Decrement
        assert!(!is_click_usage(0xB7)); // Stop
    }

    #[test]
    fn consumer_click_edge_detection() {
        // Press (Play usage present) -> edge.
        assert_eq!(consumer_click_edge(false, &[0xB0]), (true, true));
        // Same report repeated while held -> no new edge.
        assert_eq!(consumer_click_edge(true, &[0xB0]), (false, true));
        // Release report (empty) -> no edge, state cleared.
        assert_eq!(consumer_click_edge(true, &[]), (false, false));
        // Volume rotation reports never count as a click.
        assert_eq!(consumer_click_edge(false, &[0xE9]), (false, false));
        // Alternating Play then Pause presses (AVRCP toggles its own state) both register.
        let (e1, d1) = consumer_click_edge(false, &[0xB0]);
        let (_, d2) = consumer_click_edge(d1, &[]);
        let (e3, _) = consumer_click_edge(d2, &[0xB1]);
        assert!(e1 && e3);
    }

    #[test]
    fn brightness_stepping() {
        assert_eq!(brightness_step_target(50, 1), 55);
        assert_eq!(brightness_step_target(50, -1), 45);
        assert_eq!(brightness_step_target(98, 1), 100);
        assert_eq!(brightness_step_target(2, -1), 0);
        assert_eq!(brightness_step_target(50, 7), 55);
    }

    #[test]
    fn rapid_brightness_steps_accumulate_from_cache() {
        // Simulates five fast detents: each step must build on the previous target,
        // not on a stale hardware read that still says 50%.
        let stale_hardware = 50;
        let mut cached = 50;
        let mut last_set = 0u64;
        for i in 0..5u64 {
            let now = 100_000 + i * 120;
            let base = if brightness_cache_is_fresh(now, last_set) { cached } else { stale_hardware };
            cached = brightness_step_target(base, 1);
            last_set = now;
        }
        assert_eq!(cached, 75);
    }

    #[test]
    fn brightness_cache_expires() {
        assert!(!brightness_cache_is_fresh(10_000, 0));
        assert!(brightness_cache_is_fresh(10_000, 9_000));
        assert!(!brightness_cache_is_fresh(12_500, 10_000));
    }

    #[test]
    fn remote_commands_map_to_click_or_cancel() {
        assert!(remote_command_is_click(RemoteCommand::Play));
        assert!(remote_command_is_click(RemoteCommand::Pause));
        assert!(remote_command_is_click(RemoteCommand::Toggle));
        assert!(!remote_command_is_click(RemoteCommand::Next));
        assert!(!remote_command_is_click(RemoteCommand::Previous));
    }

    #[test]
    fn alternating_earbud_presses_are_each_delivered() {
        // Earbuds send PAUSE, PLAY, PAUSE...; after each we must report the state that makes
        // the next one a real change (never a no-op the system would drop).
        let mut playing = true;
        for cmd in [RemoteCommand::Pause, RemoteCommand::Play, RemoteCommand::Pause, RemoteCommand::Play] {
            let expected_change = match cmd {
                RemoteCommand::Pause => playing,
                RemoteCommand::Play => !playing,
                _ => unreachable!(),
            };
            assert!(expected_change, "{cmd:?} would be a no-op");
            playing = reported_playing_after(cmd, playing);
        }
        assert!(!reported_playing_after(RemoteCommand::Toggle, true));
        assert!(reported_playing_after(RemoteCommand::Next, true));
    }
}
