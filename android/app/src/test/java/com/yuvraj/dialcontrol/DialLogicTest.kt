package com.yuvraj.dialcontrol

import com.yuvraj.dialcontrol.dial.DialLogic
import com.yuvraj.dialcontrol.dial.DialLogic.ClickAction
import com.yuvraj.dialcontrol.dial.DialLogic.GestureAction
import com.yuvraj.dialcontrol.dial.DialLogic.MultiPress
import com.yuvraj.dialcontrol.dial.DialLogic.RemoteCommand
import com.yuvraj.dialcontrol.dial.DialLogic.VolumeAction
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Mirrors the gesture-model tests in `src-tauri/src/platform/dial_logic.rs`. */
class DialLogicTest {
    @Test
    fun clickPlaysPauseWhenHudClosedAndConfirmsWhenOpen() {
        assertEquals(ClickAction.PLAY_PAUSE, DialLogic.classifyClick(5000, 0, isHudOpen = false, isScrollMode = false))
        assertEquals(ClickAction.CONFIRM, DialLogic.classifyClick(5000, 0, isHudOpen = true, isScrollMode = false))
        assertEquals(ClickAction.EXIT_SCROLL, DialLogic.classifyClick(5000, 0, isHudOpen = false, isScrollMode = true))
    }

    @Test
    fun clickWithinDebounceIsDropped() {
        assertEquals(ClickAction.DEBOUNCED, DialLogic.classifyClick(1300, 1000, isHudOpen = true, isScrollMode = false))
        assertEquals(ClickAction.CONFIRM, DialLogic.classifyClick(1400, 1000, isHudOpen = true, isScrollMode = false))
    }

    @Test
    fun doublePressOpensAndTriplePressGoesBack() {
        assertEquals(GestureAction.OPEN_HUD, DialLogic.classifyMultiPress(MultiPress.DOUBLE, 5000, 0, false, false))
        assertEquals(GestureAction.PREVIOUS_TRACK, DialLogic.classifyMultiPress(MultiPress.TRIPLE, 5000, 0, false, false))
        assertEquals(GestureAction.DISMISS, DialLogic.classifyMultiPress(MultiPress.DOUBLE, 5000, 0, true, false))
        assertEquals(GestureAction.DISMISS, DialLogic.classifyMultiPress(MultiPress.TRIPLE, 5000, 0, true, false))
        assertEquals(GestureAction.EXIT_SCROLL, DialLogic.classifyMultiPress(MultiPress.DOUBLE, 5000, 0, false, true))
    }

    @Test
    fun lockedIndexKeepsHeadroom() {
        assertEquals(1, DialLogic.headroomIndex(0, 15))
        assertEquals(14, DialLogic.headroomIndex(15, 15))
        assertEquals(7, DialLogic.headroomIndex(7, 15))
    }

    /** classifyIndexChange with the HUD open, index 7 locked and last seen, no restore pending. */
    private fun change(new: Int, last: Int = 7, sinceRestore: Long = 10_000, echo: Boolean = false, hud: Boolean = true, scroll: Boolean = false) =
        DialLogic.classifyIndexChange(new, last, 7, 15, sinceRestore, echo, hud, scroll)

    @Test
    fun indexChangesWhileLockedAreTurns() {
        assertEquals(VolumeAction.Turn(1), change(8))
        assertEquals(VolumeAction.Turn(-1), change(5))
        assertEquals(VolumeAction.Turn(1), change(8, hud = false, scroll = true))
        // Our own setStreamVolume(locked) comes back onto the locked index.
        assertEquals(VolumeAction.Ignore, change(7))
    }

    @Test
    fun offByOneEchoAfterRestoreIsSwallowedOnce() {
        assertEquals(VolumeAction.Echo, change(8, sinceRestore = 80, echo = true))
        assertEquals(VolumeAction.Echo, change(6, sinceRestore = 80, echo = true))
        // Only one echo per restore: the next change is a real turn even inside the window.
        assertEquals(VolumeAction.Turn(1), change(8, sinceRestore = 120, echo = false))
        // Outside the window it is a turn, and a bigger jump is never an echo.
        assertEquals(VolumeAction.Turn(1), change(8, sinceRestore = 400, echo = true))
        assertEquals(VolumeAction.Turn(1), change(9, sinceRestore = 80, echo = true))
    }

    @Test
    fun turnRightAfterAnEchoCountsFromTheEchoedIndex() {
        // Echo left the phone at 8; a real CCW detent lands back on 7.
        assertEquals(VolumeAction.Turn(-1), change(7, last = 8))
    }

    @Test
    fun volumePassesThroughWhenHudClosedAndMirrorsInVolumeMode() {
        assertEquals(VolumeAction.Ignore, change(9, hud = false))
        assertEquals(VolumeAction.NaturalChange(60), DialLogic.classifyIndexChange(9, 8, -1, 15, 10_000, false, true, false))
    }

    /** Replays a dial session through the same state updates VolumeWatcher makes. */
    @Test
    fun simulatedSessionWithEchoesStepsOncePerDetent() {
        val locked = 7
        var last = locked
        var restoreAt = -10_000L
        var echoPending = false
        val turns = mutableListOf<Int>()
        // (time ms, index): CW detent, our restore, earbud echo off by one, CW, restore, CCW, restore.
        val events = listOf(0L to 8, 5L to 7, 60L to 8, 900L to 9, 905L to 7, 1800L to 6, 1805L to 7)
        for ((t, idx) in events) {
            when (val a = DialLogic.classifyIndexChange(idx, last, locked, 15, t - restoreAt, echoPending, true, false)) {
                is VolumeAction.Turn -> {
                    turns += a.direction
                    last = locked
                    restoreAt = t
                    echoPending = true
                }
                is VolumeAction.Echo -> {
                    echoPending = false
                    last = idx
                }
                else -> {}
            }
        }
        assertEquals(listOf(1, 1, -1), turns)
    }

    @Test
    fun reportedStateMirrorsEarbuds() {
        assertTrue(DialLogic.reportedPlayingAfter(RemoteCommand.PLAY, false))
        assertFalse(DialLogic.reportedPlayingAfter(RemoteCommand.PAUSE, true))
        assertFalse(DialLogic.reportedPlayingAfter(RemoteCommand.TOGGLE, true))
        assertTrue(DialLogic.reportedPlayingAfter(RemoteCommand.NEXT, true))
        assertTrue(DialLogic.remoteCommandIsClick(RemoteCommand.TOGGLE))
        assertFalse(DialLogic.remoteCommandIsClick(RemoteCommand.NEXT))
    }

    @Test
    fun brightnessStepsClamp() {
        assertEquals(100, DialLogic.brightnessStepTarget(98, 1))
        assertEquals(0, DialLogic.brightnessStepTarget(3, -1))
    }

    @Test
    fun reclaimOnlyWhenAnotherPlayerStartsAndNotDuringCooldown() {
        assertTrue(DialLogic.shouldReclaim(2, 1, 10_000, 0))
        assertFalse(DialLogic.shouldReclaim(1, 2, 10_000, 0))
        assertFalse(DialLogic.shouldReclaim(2, 1, 10_500, 10_000))
        assertTrue(DialLogic.shouldReclaim(2, 1, 11_600, 10_000))
    }

    @Test
    fun profileMatchesPackageInWheelOrder() {
        val order = listOf("default", "music", "media_video")
        val patterns = mapOf(
            "default" to listOf("*"),
            "music" to listOf("*spotify*", "*youtube.music*"),
            "media_video" to listOf("*youtube*"),
        )
        assertEquals("media_video", DialLogic.matchProfile(order, patterns, "com.google.android.youtube"))
        assertEquals("music", DialLogic.matchProfile(order, patterns, "com.google.android.apps.youtube.music"))
        assertEquals("music", DialLogic.matchProfile(order, patterns, "com.spotify.music"))
        assertEquals("default", DialLogic.matchProfile(order, patterns, "com.example.unknown"))
        assertEquals("default", DialLogic.matchProfile(order, patterns, null))
    }
}
