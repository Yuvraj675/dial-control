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

    sealed class VolumeAction {
        /** HUD closed, echo of our own restore, or nothing changed. */
        object Ignore : VolumeAction()

        /** Detent while locked: snap back to the locked index and rotate in [direction]. */
        data class Turn(val direction: Int) : VolumeAction()

        /** Volume mode (lock released): the change is real, only mirror it on the gauge. */
        data class NaturalChange(val pct: Int) : VolumeAction()
    }

    /**
     * Index-based counterpart of `classify_volume_change`. Our own restore arrives back as a
     * change onto [lockedIndex] and is swallowed because the delta is zero.
     */
    fun classifyIndexChange(
        newIndex: Int,
        lockedIndex: Int,
        maxIndex: Int,
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
        val delta = newIndex - lockedIndex
        if (delta == 0) return VolumeAction.Ignore
        return VolumeAction.Turn(if (delta > 0) 1 else -1)
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

    /** Wheel highlight moves one slice per detent and wraps around. */
    fun wrapIndex(current: Int, direction: Int, count: Int): Int =
        if (count <= 0) 0 else Math.floorMod(current + direction, count)

    /** CW (direction 1) scrolls content down, i.e. the finger swipes up, like a mouse wheel. */
    fun swipeUpFor(direction: Int): Boolean = direction > 0

    /**
     * Another app started media playback when the number of active media players grew. That
     * app now receives the earbuds' buttons, so the session has to be taken back.
     */
    fun shouldReclaim(activeMedia: Int, previousActiveMedia: Int, nowMs: Long, lastReclaimMs: Long, cooldownMs: Long = 1500L): Boolean =
        activeMedia > previousActiveMedia && (lastReclaimMs == 0L || nowMs - lastReclaimMs >= cooldownMs)
}
