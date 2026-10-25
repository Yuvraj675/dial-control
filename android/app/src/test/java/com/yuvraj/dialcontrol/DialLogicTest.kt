package com.yuvraj.dialcontrol

import com.yuvraj.dialcontrol.dial.DialLogic
import com.yuvraj.dialcontrol.dial.DialLogic.ClickAction
import com.yuvraj.dialcontrol.dial.DialLogic.GestureAction
import com.yuvraj.dialcontrol.dial.DialLogic.MultiPress
import com.yuvraj.dialcontrol.dial.DialLogic.RemoteCommand
import com.yuvraj.dialcontrol.dial.DialLogic.VolumeAction
import com.yuvraj.dialcontrol.profile.Profiles
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

    @Test
    fun indexChangesWhileLockedAreTurnsAndOwnRestoreIsSwallowed() {
        assertEquals(VolumeAction.Turn(1), DialLogic.classifyIndexChange(8, 7, 15, true, false))
        assertEquals(VolumeAction.Turn(-1), DialLogic.classifyIndexChange(5, 7, 15, true, false))
        // Our setStreamVolume(locked) comes back as a change onto the locked index.
        assertEquals(VolumeAction.Ignore, DialLogic.classifyIndexChange(7, 7, 15, true, false))
        assertEquals(VolumeAction.Turn(1), DialLogic.classifyIndexChange(8, 7, 15, false, true))
    }

    @Test
    fun volumePassesThroughWhenHudClosedAndMirrorsInVolumeMode() {
        assertEquals(VolumeAction.Ignore, DialLogic.classifyIndexChange(9, 7, 15, false, false))
        assertEquals(VolumeAction.NaturalChange(60), DialLogic.classifyIndexChange(9, -1, 15, true, false))
    }

    @Test
    fun simulatedTurnSequenceStepsOncePerDetent() {
        // CW, echo, CW, echo, CCW, echo: three rotations, echoes ignored.
        val events = listOf(8, 7, 8, 7, 6, 7)
        val turns = events.map { DialLogic.classifyIndexChange(it, 7, 15, true, false) }
            .filterIsInstance<VolumeAction.Turn>().map { it.direction }
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
    fun wheelWrapsBothWays() {
        assertEquals(0, DialLogic.wrapIndex(9, 1, 10))
        assertEquals(9, DialLogic.wrapIndex(0, -1, 10))
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
    fun profileMatchesPackageOrFallsBack() {
        assertEquals("video", Profiles.match(Profiles.defaults, "com.google.android.youtube").id)
        assertEquals("music", Profiles.match(Profiles.defaults, "com.spotify.music").id)
        assertEquals("default", Profiles.match(Profiles.defaults, "com.example.unknown").id)
        assertEquals("default", Profiles.match(Profiles.defaults, null).id)
    }

    @Test
    fun everyProfileHasACloseSlice() {
        for (p in Profiles.defaults) assertTrue(p.id, p.slices.any { it.id == "close" })
    }
}
