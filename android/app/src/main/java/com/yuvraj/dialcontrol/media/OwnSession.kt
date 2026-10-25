package com.yuvraj.dialcontrol.media

import android.content.Context
import android.content.Intent
import android.media.AudioAttributes
import android.media.AudioFormat
import android.media.AudioManager
import android.media.AudioPlaybackConfiguration
import android.media.AudioTrack
import android.media.MediaMetadata
import android.media.session.MediaSession
import android.media.session.PlaybackState
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.KeyEvent
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.dial.DialLogic
import com.yuvraj.dialcontrol.dial.DialLogic.RemoteCommand

/**
 * Our own media session, the Android counterpart of `windows/smtc.rs` and `macos/now_playing.rs`.
 *
 * Android delivers the earbuds' AVRCP Play / Pause / Next / Previous to the session of the app
 * that most recently started media playback. A browser or player would otherwise pause or skip
 * instead of opening the HUD, so we loop silence (without audio focus, so music keeps playing)
 * and take the session back whenever another app starts playing.
 */
class OwnSession(context: Context, private val onCommand: (RemoteCommand) -> Unit) {
    private val app = context.applicationContext
    private val audio = app.getSystemService(AudioManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private val session = MediaSession(app, "DialControl")
    private var track: AudioTrack? = null
    private var reportPlaying = true
    private var activeMedia = 0
    private var lastReclaimMs = 0L

    private val playbackCallback = object : AudioManager.AudioPlaybackCallback() {
        override fun onPlaybackConfigChanged(configs: MutableList<AudioPlaybackConfiguration>) {
            val count = configs.count { it.audioAttributes.usage == AudioAttributes.USAGE_MEDIA }
            val now = SystemClock.elapsedRealtime()
            val reclaim = DialLogic.shouldReclaim(count, activeMedia, now, lastReclaimMs)
            activeMedia = count
            if (reclaim) {
                lastReclaimMs = now
                // Let the other player finish starting, then become the most recent player again.
                main.postDelayed({ takeBack("another player started") }, 400)
            }
        }
    }

    fun start() {
        session.setCallback(object : MediaSession.Callback() {
            override fun onMediaButtonEvent(mediaButtonIntent: Intent): Boolean {
                val ev = mediaButtonIntent.keyEvent() ?: return false
                if (ev.action != KeyEvent.ACTION_DOWN || ev.repeatCount != 0) return true
                val cmd = when (ev.keyCode) {
                    KeyEvent.KEYCODE_MEDIA_PLAY -> RemoteCommand.PLAY
                    KeyEvent.KEYCODE_MEDIA_PAUSE -> RemoteCommand.PAUSE
                    KeyEvent.KEYCODE_MEDIA_PLAY_PAUSE, KeyEvent.KEYCODE_HEADSETHOOK -> RemoteCommand.TOGGLE
                    KeyEvent.KEYCODE_MEDIA_NEXT -> RemoteCommand.NEXT
                    KeyEvent.KEYCODE_MEDIA_PREVIOUS -> RemoteCommand.PREVIOUS
                    else -> return false
                }
                handle(cmd)
                return true
            }

            // Some Bluetooth stacks call the transport callbacks instead of delivering key events.
            override fun onPlay() = handle(RemoteCommand.PLAY)
            override fun onPause() = handle(RemoteCommand.PAUSE)
            override fun onSkipToNext() = handle(RemoteCommand.NEXT)
            override fun onSkipToPrevious() = handle(RemoteCommand.PREVIOUS)
        }, main)
        session.setMetadata(
            MediaMetadata.Builder()
                .putString(MediaMetadata.METADATA_KEY_TITLE, "Dial Control")
                .putString(MediaMetadata.METADATA_KEY_ARTIST, "Dial ready")
                .build(),
        )
        publishState()
        session.isActive = true
        startSilence()
        audio.registerAudioPlaybackCallback(playbackCallback, main)
        Diag.log("media session active (earbud buttons now come to Dial Control)")
    }

    fun stop() {
        audio.unregisterAudioPlaybackCallback(playbackCallback)
        main.removeCallbacksAndMessages(null)
        track?.run { runCatching { stop() }; release() }
        track = null
        session.isActive = false
        session.release()
    }

    private fun handle(cmd: RemoteCommand) {
        reportPlaying = DialLogic.reportedPlayingAfter(cmd, reportPlaying)
        publishState()
        Diag.log("earbud button $cmd")
        onCommand(cmd)
    }

    /**
     * Become the media button session again. Restarting playback makes us the most recent
     * player; re-publishing the same state alone is ignored (same finding as on macOS).
     */
    fun takeBack(reason: String) {
        Diag.log("reclaiming media session ($reason)")
        track?.run { runCatching { stop() }; release() }
        startSilence()
        reportPlaying = !reportPlaying
        publishState()
        reportPlaying = !reportPlaying
        publishState()
    }

    private fun publishState() {
        val state = if (reportPlaying) PlaybackState.STATE_PLAYING else PlaybackState.STATE_PAUSED
        session.setPlaybackState(
            PlaybackState.Builder()
                .setActions(
                    PlaybackState.ACTION_PLAY or PlaybackState.ACTION_PAUSE or PlaybackState.ACTION_PLAY_PAUSE or
                        PlaybackState.ACTION_SKIP_TO_NEXT or PlaybackState.ACTION_SKIP_TO_PREVIOUS,
                )
                .setState(state, 0L, 1f)
                .build(),
        )
    }

    private fun startSilence() {
        val rate = 8000
        val frames = rate // one second, looped forever
        val t = AudioTrack.Builder()
            .setAudioAttributes(
                AudioAttributes.Builder()
                    .setUsage(AudioAttributes.USAGE_MEDIA)
                    .setContentType(AudioAttributes.CONTENT_TYPE_MUSIC)
                    .build(),
            )
            .setAudioFormat(
                AudioFormat.Builder()
                    .setEncoding(AudioFormat.ENCODING_PCM_16BIT)
                    .setSampleRate(rate)
                    .setChannelMask(AudioFormat.CHANNEL_OUT_MONO)
                    .build(),
            )
            .setTransferMode(AudioTrack.MODE_STATIC)
            .setBufferSizeInBytes(frames * 2)
            .build()
        t.write(ShortArray(frames), 0, frames)
        t.setLoopPoints(0, frames, -1)
        t.play()
        track = t
    }

    @Suppress("DEPRECATION")
    private fun Intent.keyEvent(): KeyEvent? =
        if (android.os.Build.VERSION.SDK_INT >= 33) {
            getParcelableExtra(Intent.EXTRA_KEY_EVENT, KeyEvent::class.java)
        } else {
            getParcelableExtra(Intent.EXTRA_KEY_EVENT)
        }
}
