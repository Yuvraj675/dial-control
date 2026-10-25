package com.yuvraj.dialcontrol

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.provider.Settings
import android.widget.Toast
import com.yuvraj.dialcontrol.a11y.DialA11yService
import com.yuvraj.dialcontrol.actions.Actions
import com.yuvraj.dialcontrol.dial.DialLogic
import com.yuvraj.dialcontrol.dial.DialLogic.ClickAction
import com.yuvraj.dialcontrol.dial.DialLogic.GestureAction
import com.yuvraj.dialcontrol.dial.DialLogic.MultiPress
import com.yuvraj.dialcontrol.dial.DialLogic.RemoteCommand
import com.yuvraj.dialcontrol.dial.VolumeWatcher
import com.yuvraj.dialcontrol.media.OwnSession
import com.yuvraj.dialcontrol.media.PlayerForwarder
import com.yuvraj.dialcontrol.profile.Profile
import com.yuvraj.dialcontrol.profile.Profiles
import com.yuvraj.dialcontrol.profile.Slice
import com.yuvraj.dialcontrol.profile.SliceType

/**
 * Gesture model shared with the desktop app (CLAUDE.md §5.1). Every caller (media session
 * callbacks, volume broadcasts, accessibility events) runs on the main thread, so the state
 * here needs no locking.
 */
object DialController {
    enum class Mode { MENU, VOLUME, BRIGHTNESS }

    private const val AUTO_CLOSE_MS = 6000L
    private const val HIDE_SETTLE_MS = 150L

    private val main = Handler(Looper.getMainLooper())

    var a11y: DialA11yService? = null
    var session: OwnSession? = null
    var volume: VolumeWatcher? = null

    var foregroundPackage: String? = null
    val profiles: List<Profile> = Profiles.defaults

    var isHudOpen = false
        private set
    var isScrollMode = false
        private set
    var mode = Mode.MENU
        private set
    var selected = 0
        private set
    var gaugePct = 0
        private set
    var profile: Profile = Profiles.match(profiles, null)
        private set

    private var lastClickMs = 0L
    private var lastGestureMs = 0L
    private val autoClose = Runnable { if (isHudOpen) hideHud() }

    fun onRemoteCommand(cmd: RemoteCommand) {
        when (cmd) {
            RemoteCommand.NEXT -> onMultiPress(MultiPress.DOUBLE)
            RemoteCommand.PREVIOUS -> onMultiPress(MultiPress.TRIPLE)
            else -> onClick()
        }
    }

    private fun onClick() {
        val now = SystemClock.elapsedRealtime()
        val action = DialLogic.classifyClick(now, lastClickMs, isHudOpen, isScrollMode)
        if (action != ClickAction.DEBOUNCED) lastClickMs = now
        Diag.log("click -> $action")
        when (action) {
            ClickAction.DEBOUNCED -> {}
            ClickAction.EXIT_SCROLL -> exitScroll()
            ClickAction.PLAY_PAUSE -> forward(PlayerForwarder.Command.PLAY_PAUSE)
            ClickAction.CONFIRM -> if (mode == Mode.MENU) runSelected() else hideHud()
        }
    }

    private fun onMultiPress(gesture: MultiPress) {
        val now = SystemClock.elapsedRealtime()
        val action = DialLogic.classifyMultiPress(gesture, now, lastGestureMs, isHudOpen, isScrollMode)
        if (action != GestureAction.DEBOUNCED) lastGestureMs = now
        Diag.log("$gesture press -> $action")
        when (action) {
            GestureAction.DEBOUNCED -> {}
            GestureAction.OPEN_HUD -> showHud()
            GestureAction.DISMISS -> hideHud()
            GestureAction.EXIT_SCROLL -> exitScroll()
            GestureAction.PREVIOUS_TRACK -> forward(PlayerForwarder.Command.PREVIOUS)
        }
    }

    fun onRotate(direction: Int) {
        if (isScrollMode) {
            a11y?.swipe(up = DialLogic.swipeUpFor(direction), fraction = 0.22f, durationMs = 180)
            return
        }
        if (!isHudOpen) return
        when (mode) {
            Mode.MENU -> selected = DialLogic.wrapIndex(selected, direction, profile.slices.size)
            Mode.BRIGHTNESS -> gaugePct = Actions.stepBrightness(appContext() ?: return, direction)
            Mode.VOLUME -> {}
        }
        a11y?.refreshWheel(tick = true)
        rescheduleAutoClose()
    }

    fun onNaturalVolume(pct: Int) {
        gaugePct = pct
        a11y?.refreshWheel(tick = true)
        rescheduleAutoClose()
    }

    fun toggleHud() = if (isHudOpen) hideHud() else showHud()

    fun showHud() {
        val service = a11y
        if (service == null) {
            Diag.log("HUD needs the Dial Control accessibility service")
            toast("Enable Dial Control in Accessibility settings")
            return
        }
        profile = Profiles.match(profiles, foregroundPackage)
        selected = 0
        mode = Mode.MENU
        isHudOpen = true
        volume?.lock()
        service.showWheel()
        session?.takeBack("HUD opened")
        rescheduleAutoClose()
        Diag.log("HUD shown, profile ${profile.name} (${foregroundPackage ?: "unknown app"})")
    }

    /** Close the HUD, then run [after] once the overlay is gone. */
    fun hideHud(after: (() -> Unit)? = null) {
        main.removeCallbacks(autoClose)
        if (isHudOpen) {
            isHudOpen = false
            a11y?.hideWheel()
            if (!isScrollMode) volume?.unlock()
        }
        after?.let { main.postDelayed(it, HIDE_SETTLE_MS) }
    }

    /** Touch on the overlay: pick a slice directly. */
    fun selectAndRun(index: Int) {
        if (!isHudOpen || index !in profile.slices.indices) return
        selected = index
        runSelected()
    }

    private fun runSelected() {
        val slice = profile.slices.getOrNull(selected) ?: return hideHud()
        Diag.log("run slice ${slice.id}")
        when (slice.type) {
            SliceType.CLOSE -> hideHud()
            SliceType.MODE -> enterMode(slice)
            else -> {
                val service = a11y
                hideHud { if (service != null) Actions.run(service, slice) }
            }
        }
    }

    private fun enterMode(slice: Slice) {
        val ctx = appContext() ?: return
        when (slice.action) {
            "scroll" -> enterScroll()
            "volume" -> {
                mode = Mode.VOLUME
                volume?.release()
                gaugePct = volume?.volumePct ?: 0
            }
            "brightness" -> {
                if (!Settings.System.canWrite(ctx)) {
                    toast("Allow Dial Control to modify system settings")
                    return hideHud()
                }
                mode = Mode.BRIGHTNESS
                gaugePct = Actions.brightnessPct(ctx)
            }
        }
        a11y?.refreshWheel(tick = false)
        rescheduleAutoClose()
    }

    private fun enterScroll() {
        isScrollMode = true
        hideHud()
        DialService.instance?.updateNotification("Scroll mode: turn to scroll, click to exit")
        toast("Scroll mode: click the dial to exit")
        Diag.log("scroll mode on")
    }

    fun exitScroll() {
        if (!isScrollMode) return
        isScrollMode = false
        volume?.unlock()
        DialService.instance?.updateNotification(null)
        toast("Scroll mode off")
        Diag.log("scroll mode off")
    }

    private fun forward(cmd: PlayerForwarder.Command) {
        val ctx = appContext() ?: return
        if (!PlayerForwarder.forward(ctx, cmd)) toast("Grant notification access to control your player")
    }

    private fun rescheduleAutoClose() {
        main.removeCallbacks(autoClose)
        if (isHudOpen) main.postDelayed(autoClose, AUTO_CLOSE_MS)
    }

    private fun appContext() = (a11y ?: DialService.instance)?.applicationContext

    private fun toast(msg: String) {
        appContext()?.let { Toast.makeText(it, msg, Toast.LENGTH_SHORT).show() }
    }
}
