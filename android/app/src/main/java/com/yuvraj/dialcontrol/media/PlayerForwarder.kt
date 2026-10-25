package com.yuvraj.dialcontrol.media

import android.content.ComponentName
import android.content.Context
import android.media.session.MediaController
import android.media.session.MediaSessionManager
import android.media.session.PlaybackState
import android.service.notification.NotificationListenerService
import com.yuvraj.dialcontrol.Diag

/**
 * Sends play/pause/next/previous to the real player (the counterpart of `smtc::forward_media`).
 * A synthetic media key would come straight back to our own session, so we drive the other
 * app's MediaController instead. Listing sessions requires notification access.
 */
object PlayerForwarder {
    enum class Command { PLAY_PAUSE, NEXT, PREVIOUS }

    fun hasAccess(context: Context): Boolean =
        runCatching { controllers(context); true }.getOrDefault(false)

    fun forward(context: Context, cmd: Command): Boolean {
        val list = runCatching { controllers(context) }.getOrElse {
            Diag.log("cannot reach the player: grant notification access")
            return false
        }
        val own = context.packageName
        val foreign = list.filter { it.packageName != own }
        val target = foreign.firstOrNull { it.playbackState?.state == PlaybackState.STATE_PLAYING }
            ?: foreign.firstOrNull()
        if (target == null) {
            Diag.log("no other media session to control")
            return false
        }
        val t = target.transportControls
        when (cmd) {
            Command.PLAY_PAUSE ->
                if (target.playbackState?.state == PlaybackState.STATE_PLAYING) t.pause() else t.play()
            Command.NEXT -> t.skipToNext()
            Command.PREVIOUS -> t.skipToPrevious()
        }
        Diag.log("forwarded $cmd to ${target.packageName}")
        return true
    }

    private fun controllers(context: Context): List<MediaController> {
        val mgr = context.getSystemService(MediaSessionManager::class.java)
        return mgr.getActiveSessions(ComponentName(context, MediaListener::class.java))
    }
}

/** Exists only so the user can grant notification access, which unlocks `getActiveSessions`. */
class MediaListener : NotificationListenerService()
