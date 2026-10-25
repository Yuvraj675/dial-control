package com.yuvraj.dialcontrol

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.IBinder
import com.yuvraj.dialcontrol.dial.VolumeWatcher
import com.yuvraj.dialcontrol.media.OwnSession

/**
 * Foreground service that owns the dial listeners: our media session (clicks and multi-presses)
 * and the volume watcher (turns). Started from the main screen or by the accessibility service
 * when the system binds it at boot, which is exempt from background-start limits.
 */
class DialService : Service() {
    private var session: OwnSession? = null
    private var volume: VolumeWatcher? = null

    override fun onCreate() {
        super.onCreate()
        instance = this
        createChannel()
        val notification = buildNotification(null)
        if (Build.VERSION.SDK_INT >= 29) {
            startForeground(NOTIFICATION_ID, notification, ServiceInfo.FOREGROUND_SERVICE_TYPE_MEDIA_PLAYBACK)
        } else {
            startForeground(NOTIFICATION_ID, notification)
        }
        volume = VolumeWatcher(
            this,
            isHudOpen = { DialController.isHudOpen },
            isScrollMode = { DialController.isScrollMode },
            onTurn = DialController::onRotate,
            onNaturalChange = DialController::onNaturalVolume,
        ).also { it.start(); DialController.volume = it }
        session = OwnSession(this, DialController::onRemoteCommand).also { it.start(); DialController.session = it }
        Diag.log("dial service started")
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.action == ACTION_STOP) {
            stopSelf()
            return START_NOT_STICKY
        }
        return START_STICKY
    }

    override fun onDestroy() {
        DialController.exitScroll()
        DialController.hideHud()
        session?.stop()
        volume?.stop()
        DialController.session = null
        DialController.volume = null
        instance = null
        Diag.log("dial service stopped")
        super.onDestroy()
    }

    override fun onBind(intent: Intent?): IBinder? = null

    fun updateNotification(text: String?) {
        getSystemService(NotificationManager::class.java).notify(NOTIFICATION_ID, buildNotification(text))
    }

    private fun createChannel() {
        val channel = NotificationChannel(CHANNEL_ID, "Dial listener", NotificationManager.IMPORTANCE_LOW)
        channel.description = "Shown while Dial Control listens to the earbud dial"
        getSystemService(NotificationManager::class.java).createNotificationChannel(channel)
    }

    private fun buildNotification(text: String?): Notification {
        val open = PendingIntent.getActivity(
            this, 0, Intent(this, MainActivity::class.java), PendingIntent.FLAG_IMMUTABLE,
        )
        val stop = PendingIntent.getService(
            this, 1, Intent(this, DialService::class.java).setAction(ACTION_STOP), PendingIntent.FLAG_IMMUTABLE,
        )
        return Notification.Builder(this, CHANNEL_ID)
            .setSmallIcon(R.drawable.ic_dial)
            .setContentTitle("Dial Control")
            .setContentText(text ?: "Double press the dial to open the wheel")
            .setContentIntent(open)
            .setOngoing(true)
            .addAction(Notification.Action.Builder(null, "Stop", stop).build())
            .build()
    }

    companion object {
        private const val CHANNEL_ID = "dial"
        private const val NOTIFICATION_ID = 1
        private const val ACTION_STOP = "com.yuvraj.dialcontrol.STOP"

        var instance: DialService? = null
            private set

        fun start(context: Context) {
            context.startForegroundService(Intent(context, DialService::class.java))
        }

        fun stop(context: Context) {
            context.stopService(Intent(context, DialService::class.java))
        }
    }
}
