package com.yuvraj.dialcontrol.dial

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioManager
import android.os.Build
import android.os.SystemClock
import com.yuvraj.dialcontrol.Diag

/**
 * Detects dial turns. With AVRCP Absolute Volume the earbuds set the phone's music volume
 * directly, so a turn shows up as a STREAM_MUSIC index change. While the HUD (or scroll mode)
 * is active the index is locked: each change away from it is one detent and is snapped back,
 * and the earbuds' answer to that snap-back is filtered as an echo (`DialLogic.classifyIndexChange`).
 */
class VolumeWatcher(
    context: Context,
    private val isHudOpen: () -> Boolean,
    private val isScrollMode: () -> Boolean,
    private val onTurn: (Int) -> Unit,
    private val onNaturalChange: (Int) -> Unit,
) {
    private val app = context.applicationContext
    private val audio = app.getSystemService(AudioManager::class.java)

    private var lockedIndex = -1
    private var lastIndex = -1
    private var restoreAt = 0L
    private var echoPending = false

    /** The user's listening volume, restored when the lock ends; -1 when nothing to restore. */
    private var userIndex = -1

    private val receiver = object : BroadcastReceiver() {
        override fun onReceive(c: Context, intent: Intent) {
            if (intent.getIntExtra(EXTRA_STREAM_TYPE, -1) != AudioManager.STREAM_MUSIC) return
            val index = intent.getIntExtra(EXTRA_STREAM_VALUE, -1)
            if (index < 0) return
            handle(index)
        }
    }

    fun start() {
        val filter = IntentFilter(VOLUME_CHANGED_ACTION)
        if (Build.VERSION.SDK_INT >= 33) {
            app.registerReceiver(receiver, filter, Context.RECEIVER_NOT_EXPORTED)
        } else {
            app.registerReceiver(receiver, filter)
        }
    }

    fun stop() {
        runCatching { app.unregisterReceiver(receiver) }
        unlock()
    }

    private val maxIndex get() = audio.getStreamMaxVolume(AudioManager.STREAM_MUSIC)
    private val currentIndex get() = audio.getStreamVolume(AudioManager.STREAM_MUSIC)

    private fun pct(index: Int) = if (maxIndex > 0) index * 100 / maxIndex else 0

    /** The user's real volume (not the headroom position while locked). */
    val volumePct: Int get() = pct(if (userIndex >= 0) userIndex else currentIndex)

    private fun handle(index: Int) {
        val since = SystemClock.elapsedRealtime() - restoreAt
        val action = DialLogic.classifyIndexChange(
            index, lastIndex, lockedIndex, maxIndex, since, echoPending, isHudOpen(), isScrollMode(),
        )
        when (action) {
            is DialLogic.VolumeAction.Ignore ->
                if (!isHudOpen() && !isScrollMode()) Diag.log("volume $index/$maxIndex (passes through)")
            is DialLogic.VolumeAction.Echo -> {
                echoPending = false
                lastIndex = index
                Diag.log("volume echo $index (locked $lockedIndex, ${since}ms after restore) ignored")
            }
            is DialLogic.VolumeAction.Turn -> {
                Diag.log("dial turn ${if (action.direction > 0) "CW" else "CCW"} (index $index, last $lastIndex, locked $lockedIndex)")
                restoreLocked()
                onTurn(action.direction)
            }
            is DialLogic.VolumeAction.NaturalChange -> {
                lastIndex = index
                onNaturalChange(action.pct)
            }
        }
    }

    private fun restoreLocked() {
        lastIndex = lockedIndex
        restoreAt = SystemClock.elapsedRealtime()
        echoPending = true
        setIndex(lockedIndex)
    }

    /** Freeze the volume (with headroom) so turns become navigation. Returns the user's volume %. */
    fun lock(): Int {
        if (lockedIndex < 0) {
            val cur = currentIndex
            if (userIndex < 0) userIndex = cur
            lockedIndex = DialLogic.headroomIndex(cur, maxIndex)
            lastIndex = cur
            if (lockedIndex != cur) restoreLocked()
        }
        return pct(userIndex)
    }

    /** Volume mode: turns change the volume for real; restores the user's level first. Returns it in %. */
    fun release(): Int {
        val user = if (userIndex >= 0) userIndex else currentIndex
        lockedIndex = -1
        userIndex = -1
        lastIndex = user
        if (user != currentIndex) setIndex(user)
        return pct(user)
    }

    /** End the lock and put back the user's listening volume. */
    fun unlock() {
        val restore = userIndex
        lockedIndex = -1
        userIndex = -1
        echoPending = false
        if (restore >= 0 && restore != currentIndex) setIndex(restore)
    }

    fun setPct(pct: Int) {
        setIndex((pct.coerceIn(0, 100) * maxIndex + 50) / 100)
    }

    private fun setIndex(index: Int) {
        // Flags 0: no system volume panel for our own changes.
        runCatching { audio.setStreamVolume(AudioManager.STREAM_MUSIC, index, 0) }
    }

    private companion object {
        // Not in the public SDK, but sent by AudioService on every Android release since 4.x.
        const val VOLUME_CHANGED_ACTION = "android.media.VOLUME_CHANGED_ACTION"
        const val EXTRA_STREAM_TYPE = "android.media.EXTRA_VOLUME_STREAM_TYPE"
        const val EXTRA_STREAM_VALUE = "android.media.EXTRA_VOLUME_STREAM_VALUE"
    }
}
