package com.yuvraj.dialcontrol.actions

import android.accessibilityservice.AccessibilityService
import android.app.NotificationManager
import android.content.Context
import android.content.Intent
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CameraManager
import android.media.AudioManager
import android.net.Uri
import android.provider.Settings
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.a11y.DialA11yService
import com.yuvraj.dialcontrol.dial.DialLogic

/**
 * Phone actions behind `system`, `launch` and `mute` slices (counterpart of
 * `src-tauri/src/actions.rs`). The list of `system` actions lives in `src/data/androidProfiles.ts`.
 */
object Actions {
    private var torchOn = false

    fun runSystem(service: DialA11yService, action: String) {
        runCatching {
            when (action) {
                "swipe_up" -> service.swipe(up = true, fraction = 0.5f, durationMs = 220)
                "swipe_down" -> service.swipe(up = false, fraction = 0.5f, durationMs = 220)
                "torch" -> toggleTorch(service)
                "dnd" -> toggleDnd(service)
                else -> globalAction(action)?.let { service.performGlobalAction(it) }
                    ?: Diag.log("phone action $action is not available here")
            }
        }.onFailure { Diag.log("phone action $action failed: ${it.message}") }
    }

    private fun globalAction(name: String): Int? = when (name) {
        "back" -> AccessibilityService.GLOBAL_ACTION_BACK
        "home" -> AccessibilityService.GLOBAL_ACTION_HOME
        "recents" -> AccessibilityService.GLOBAL_ACTION_RECENTS
        "notifications" -> AccessibilityService.GLOBAL_ACTION_NOTIFICATIONS
        "quick_settings" -> AccessibilityService.GLOBAL_ACTION_QUICK_SETTINGS
        "power" -> AccessibilityService.GLOBAL_ACTION_POWER_DIALOG
        "split_screen" -> AccessibilityService.GLOBAL_ACTION_TOGGLE_SPLIT_SCREEN
        "lock" -> if (android.os.Build.VERSION.SDK_INT >= 28) AccessibilityService.GLOBAL_ACTION_LOCK_SCREEN else null
        "screenshot" -> if (android.os.Build.VERSION.SDK_INT >= 28) AccessibilityService.GLOBAL_ACTION_TAKE_SCREENSHOT else null
        else -> null
    }

    /** `target` is a package name (from the app picker) or a URL. */
    fun launch(context: Context, target: String) {
        val intent = if (target.contains("://")) {
            Intent(Intent.ACTION_VIEW, Uri.parse(target))
        } else {
            context.packageManager.getLaunchIntentForPackage(target)
        }
        if (intent == null) {
            Diag.log("launch: $target is not installed")
            return
        }
        runCatching { context.startActivity(intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
            .onFailure { Diag.log("launch $target failed: ${it.message}") }
    }

    fun toggleMute(context: Context) {
        context.getSystemService(AudioManager::class.java)
            .adjustStreamVolume(AudioManager.STREAM_MUSIC, AudioManager.ADJUST_TOGGLE_MUTE, AudioManager.FLAG_SHOW_UI)
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

    fun setBrightness(context: Context, pct: Int): Int {
        if (!Settings.System.canWrite(context)) {
            Diag.log("brightness needs the modify system settings permission")
            return brightnessPct(context)
        }
        val cr = context.contentResolver
        Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS_MODE, Settings.System.SCREEN_BRIGHTNESS_MODE_MANUAL)
        Settings.System.putInt(cr, Settings.System.SCREEN_BRIGHTNESS, Math.round(pct.coerceIn(0, 100) * 2.55f).coerceIn(1, 255))
        return pct
    }

    /** Steps the system brightness by 5% (manual mode) and returns the new percentage. */
    fun stepBrightness(context: Context, direction: Int): Int =
        setBrightness(context, DialLogic.brightnessStepTarget(brightnessPct(context), direction))
}
