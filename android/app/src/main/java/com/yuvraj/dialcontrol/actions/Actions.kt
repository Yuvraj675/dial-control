package com.yuvraj.dialcontrol.actions

import android.accessibilityservice.AccessibilityService
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.media.AudioManager
import android.os.Build
import android.provider.Settings
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.a11y.DialA11yService
import com.yuvraj.dialcontrol.dial.DialLogic
import com.yuvraj.dialcontrol.media.PlayerForwarder
import com.yuvraj.dialcontrol.profile.Slice
import com.yuvraj.dialcontrol.profile.SliceType

/** Runs a slice after the HUD has closed (counterpart of `src-tauri/src/actions.rs`). */
object Actions {
    private var torchOn = false

    fun run(service: DialA11yService, slice: Slice) {
        runCatching {
            when (slice.type) {
                SliceType.MEDIA -> PlayerForwarder.forward(
                    service,
                    when (slice.action) {
                        "next" -> PlayerForwarder.Command.NEXT
                        "prev" -> PlayerForwarder.Command.PREVIOUS
                        else -> PlayerForwarder.Command.PLAY_PAUSE
                    },
                )
                SliceType.GLOBAL -> globalAction(slice.action)?.let { service.performGlobalAction(it) }
                    ?: Diag.log("global action ${slice.action} not available on this Android version")
                SliceType.SWIPE -> when (slice.action) {
                    "left", "right" -> service.swipeHorizontal(slice.action == "left", 0.6f, 220)
                    else -> service.swipe(up = slice.action != "down", fraction = 0.5f, durationMs = 220)
                }
                SliceType.LAUNCH -> launch(service, slice.target)
                SliceType.TORCH -> toggleTorch(service)
                SliceType.MUTE -> service.getSystemService(AudioManager::class.java)
                    .adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_TOGGLE_MUTE, AudioManager.FLAG_SHOW_UI)
                SliceType.DND -> toggleDnd(service)
                SliceType.MODE, SliceType.CLOSE -> {}
            }
        }.onFailure { Diag.log("slice ${slice.id} failed: ${it.message}") }
    }

    private fun globalAction(name: String): Int? = when (name) {
        "back" -> AccessibilityService.GLOBAL_ACTION_BACK
        "home" -> AccessibilityService.GLOBAL_ACTION_HOME
        "recents" -> AccessibilityService.GLOBAL_ACTION_RECENTS
        "notifications" -> AccessibilityService.GLOBAL_ACTION_NOTIFICATIONS
        "quick_settings" -> AccessibilityService.GLOBAL_ACTION_QUICK_SETTINGS
        "power" -> AccessibilityService.GLOBAL_ACTION_POWER_DIALOG
        "split_screen" -> AccessibilityService.GLOBAL_ACTION_TOGGLE_SPLIT_SCREEN
        "lock" -> if (Build.VERSION.SDK_INT >= 28) AccessibilityService.GLOBAL_ACTION_LOCK_SCREEN else null
        "screenshot" -> if (Build.VERSION.SDK_INT >= 28) AccessibilityService.GLOBAL_ACTION_TAKE_SCREENSHOT else null
        else -> null
    }

    private fun launch(context: Context, pkg: String) {
        val intent = context.packageManager.getLaunchIntentForPackage(pkg)
        if (intent == null) {
            Diag.log("launch: $pkg is not installed")
            return
        }
        context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    private fun toggleTorch(context: Context) {
        val cam = context.getSystemService(CameraManager::class.java)
        val id = cam.cameraIdList.firstOrNull {
            cam.getCameraCharacteristics(it).get(CameraCharacteristics.FLASH_INFO_AVAILABLE) == true
        } ?: return Diag.log("no flashlight")
        torchOn = !torchOn
        cam.setTorchMode(id, torchOn)
    }

    private fun toggleDnd(context: Context) {
        val nm = context.getSystemService(NotificationManager::class.java)
        if (!nm.isNotificationPolicyAccessGranted) {
            Diag.log("DND needs Do Not Disturb access")
            return
        }
        val on = nm.currentInterruptionFilter != NotificationManager.INTERRUPTION_FILTER_ALL
        nm.setInterruptionFilter(
            if (on) NotificationManager.INTERRUPTION_FILTER_ALL else NotificationManager.INTERRUPTION_FILTER_PRIORITY,
        )
    }

    fun brightnessPct(context: Context): Int {
        val raw = Settings.System.getInt(context.contentResolver, Settings.System.SCREEN_BRIGHTNESS, 128)
        return Math.round(raw * 100 / 255f).coerceIn(0, 100)
    }

    /** Steps the system brightness by 5% (manual mode) and returns the new percentage. */
    fun stepBrightness(context: Context, direction: Int): Int {
        val target = DialLogic.brightnessStepTarget(brightnessPct(context), direction)
        if (!Settings.System.canWrite(context)) return target
        val cr = context.contentResolver
        Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS_MODE, Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL)
        Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS, Math.round(target * 2.55f).coerceIn(1, 255))
        return target
    }
}
