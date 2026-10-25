package com.yuvraj.dialcontrol.dial

import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.media.AudioManager
import android.os.Build
import com.yuvraj.dialcontrol.Diag

/**
 * Detects dial turns. With AVRCP Absolute Volume the earbuds set the phone's music volume
 * directly, so a turn shows up as a STREAM_MUSIC index change. While the HUD (or scroll mode)
 * is active the index is locked: each change away from it is one detent and is snapped back.
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

    @Volatile private var lockedIndex = -1

    /** The user's listening volume, restored when the lock ends; -1 when nothing to restore. */
    @Volatile private var userIndex = -1

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

    val volumePct: Int get() = if (maxIndex > 0) currentIndex * 100 / maxIndex else 0

    private fun handle(index: Int) {
        when (val a = DialLogic.classifyIndexChange(index, lockedIndex, maxIndex, isHudOpen(), isScrollMode())) {
            is DialLogic.VolumeAction.Ignore -> {}
            is DialLogic.VolumeAction.Turn -> {
                Diag.log("dial turn ${if (a.direction > 0) "CW" else "CCW"} (index $index, locked $lockedIndex)")
                setIndex(lockedIndex)
                onTurn(a.direction)
            }
            is DialLogic.VolumeAction.NaturalChange -> onNaturalChange(a.pct)
        }
        if (!isHudOpen() && !isScrollMode()) Diag.log("volume $index/$maxIndex (passes through)")
    }

    /** Freeze the volume (with headroom) so turns become navigation instead of volume changes. */
    fun lock() {
        if (lockedIndex >= 0) return
        val cur = currentIndex
        if (userIndex < 0) userIndex = cur
        lockedIndex = DialLogic.headroomIndex(cur, maxIndex)
        if (lockedIndex != cur) setIndex(lockedIndex)
    }

    /** Volume mode: let turns change the volume for real and keep the result on exit. */
    fun release() {
        lockedIndex = -1
        userIndex = -1
    }

    /** End the lock and put back the user's listening volume. */
    fun unlock() {
        val restore = userIndex
        lockedIndex = -1
        userIndex = -1
        if (restore >= 0 && restore != currentIndex) setIndex(restore)
    }

    private fun setIndex(index: Int) {
        // Flags 0: no system volume panel for our own restores.
        runCatching { audio.setStreamVolume(AudioManager.STREAM_MUSIC, index, 0) }
    }

    private companion object {
        // Not in the public SDK, but sent by AudioService on every Android release since 4.x.
        const val VOLUME_CHANGED_ACTION = "android.media.VOLUME_CHANGED_ACTION"
        const val EXTRA_STREAM_TYPE = "android.media.EXTRA_VOLUME_STREAM_TYPE"
        const val EXTRA_STREAM_VALUE = "android.media.EXTRA_VOLUME_STREAM_VALUE"
    }
}
