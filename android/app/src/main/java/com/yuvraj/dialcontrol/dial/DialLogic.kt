package com.yuvraj.dialcontrol.dial

/**
 * Platform-agnostic dial decisions, ported from `src-tauri/src/platform/dial_logic.rs`.
 *
 * Every Android component (media session, volume watcher, overlay) funnels raw signals
 * through these pure functions so the behaviour is unit-tested on the JVM. Keep the
 * gesture model identical to the desktop backends (CLAUDE.md §5.1).
 */
object DialLogic {
    const val CLICK_DEBOUNCE_MS = 400L
    const val BRIGHTNESS_STEP = 5

    enum class ClickAction { DEBOUNCED, EXIT_SCROLL, PLAY_PAUSE, CONFIRM }

    enum class MultiPress { DOUBLE, TRIPLE }

    enum class GestureAction { DEBOUNCED, OPEN_HUD, DISMISS, EXIT_SCROLL, PREVIOUS_TRACK }

    fun classifyClick(
        nowMs: Long,
        lastClickMs: Long,
        isHudOpen: Boolean,
        isScrollMode: Boolean,
        debounceMs: Long = CLICK_DEBOUNCE_MS,
    ): ClickAction = when {
        lastClickMs != 0L && nowMs - lastClickMs < debounceMs -> ClickAction.DEBOUNCED
        isScrollMode -> ClickAction.EXIT_SCROLL
        !isHudOpen -> ClickAction.PLAY_PAUSE
        else -> ClickAction.CONFIRM
    }

    /** The earbuds recognise double / triple presses themselves and send AVRCP Next / Previous. */
    fun classifyMultiPress(
        gesture: MultiPress,
        nowMs: Long,
        lastGestureMs: Long,
        isHudOpen: Boolean,
        isScrollMode: Boolean,
        debounceMs: Long = CLICK_DEBOUNCE_MS,
    ): GestureAction = when {
        lastGestureMs != 0L && nowMs - lastGestureMs < debounceMs -> GestureAction.DEBOUNCED
        isScrollMode -> GestureAction.EXIT_SCROLL
        isHudOpen -> GestureAction.DISMISS
        gesture == MultiPress.DOUBLE -> GestureAction.OPEN_HUD
        else -> GestureAction.PREVIOUS_TRACK
    }

    /**
     * Android exposes music volume as an integer index (0..max, usually 15). The locked index
     * keeps one step of headroom on both sides so a turn always registers in either direction.
     */
    fun headroomIndex(current: Int, max: Int): Int =
        if (max < 2) current else current.coerceIn(1, max - 1)

    /**
     * After we snap the volume back, the earbuds answer with their own rounded value, which can
     * land one index off the locked one (AVRCP 0..127 vs a 0..15 index). Within this window such
     * a change is that echo, not a detent. Treating it as a turn snapped back again and looped:
     * the volume kept changing and the wheel moved by itself (same failure as CLAUDE.md §6.4).
     * Only one echo is accepted per restore, so a fast real turn right after it still counts.
     */
    const val VOLUME_ECHO_WINDOW_MS = 250L

    sealed class VolumeAction {
        /** HUD closed or nothing changed. */
        object Ignore : VolumeAction()

        /** Echo of our own restore: track the index, no rotation, no new restore. */
        object Echo : VolumeAction()

        /** Detent while locked: snap back to the locked index and rotate in [direction]. */
        data class Turn(val direction: Int) : VolumeAction()

        /** Volume mode (lock released): the change is real, only mirror it on the gauge. */
        data class NaturalChange(val pct: Int) : VolumeAction()
    }

    /**
     * Index-based counterpart of `classify_volume_change`. The direction comes from the last
     * observed index (not the locked one), so a turn right after an off-by-one echo still counts.
     */
    fun classifyIndexChange(
        newIndex: Int,
        lastIndex: Int,
        lockedIndex: Int,
        maxIndex: Int,
        msSinceRestore: Long,
        echoPending: Boolean,
        isHudOpen: Boolean,
        isScrollMode: Boolean,
    ): VolumeAction {
        if (!isHudOpen && !isScrollMode) return VolumeAction.Ignore
        if (lockedIndex < 0) {
            if (isHudOpen && !isScrollMode && maxIndex > 0) {
                return VolumeAction.NaturalChange((newIndex * 100 / maxIndex).coerceIn(0, 100))
            }
            return VolumeAction.Ignore
        }
        if (echoPending && newIndex != lockedIndex && msSinceRestore in 0 until VOLUME_ECHO_WINDOW_MS &&
            Math.abs(newIndex - lockedIndex) <= 1
        ) {
            return VolumeAction.Echo
        }
        val delta = newIndex - lastIndex
        if (delta == 0) return VolumeAction.Ignore
        return VolumeAction.Turn(if (delta > 0) 1 else -1)
    }

    /** Glob-style `*pattern*` matched against the foreground package (cf. `patternMatches`). */
    fun patternMatchesPackage(pattern: String, packageName: String): Boolean {
        val clean = pattern.replace("*", "").trim().lowercase()
        return clean.isNotEmpty() && packageName.lowercase().contains(clean)
    }

    /** First profile in wheel order whose patterns match; "default" and "home" are fallbacks only. */
    fun matchProfile(order: List<String>, patterns: Map<String, List<String>>, packageName: String?): String {
        if (packageName != null) {
            for (id in order) {
                if (id == "default" || id == "home") continue
                if (patterns[id].orEmpty().any { patternMatchesPackage(it, packageName) }) return id
            }
        }
        return "default"
    }

    enum class RemoteCommand { PLAY, PAUSE, TOGGLE, NEXT, PREVIOUS }

    fun remoteCommandIsClick(cmd: RemoteCommand): Boolean =
        cmd == RemoteCommand.PLAY || cmd == RemoteCommand.PAUSE || cmd == RemoteCommand.TOGGLE

    /**
     * AVRCP has no toggle: the earbuds alternate PLAY / PAUSE from the state we report, so each
     * click must leave us reporting what the next (opposite) press expects.
     */
    fun reportedPlayingAfter(cmd: RemoteCommand, playing: Boolean): Boolean = when (cmd) {
        RemoteCommand.PLAY -> true
        RemoteCommand.PAUSE -> false
        RemoteCommand.TOGGLE -> !playing
        RemoteCommand.NEXT, RemoteCommand.PREVIOUS -> playing
    }

    fun brightnessStepTarget(currentPct: Int, direction: Int): Int =
        (currentPct + if (direction > 0) BRIGHTNESS_STEP else -BRIGHTNESS_STEP).coerceIn(0, 100)

    /** CW (direction 1) scrolls content down, i.e. the finger swipes up, like a mouse wheel. */
    fun swipeUpFor(direction: Int): Boolean = direction > 0

    /**
     * Another app started media playback when the number of active media players grew. That
     * app now receives the earbuds' buttons, so the session has to be taken back.
     */
    fun shouldReclaim(activeMedia: Int, previousActiveMedia: Int, nowMs: Long, lastReclaimMs: Long, cooldownMs: Long = 1500L): Boolean =
        activeMedia > previousActiveMedia && (lastReclaimMs == 0L || nowMs - lastReclaimMs >= cooldownMs)
}
