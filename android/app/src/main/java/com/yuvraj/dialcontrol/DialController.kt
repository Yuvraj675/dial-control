package com.yuvraj.dialcontrol

import android.os.Handler
import android.os.Looper
import android.os.SystemClock
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
import com.yuvraj.dialcontrol.web.WebUi
import org.json.JSONObject

/**
 * Android stand-in for the desktop backend state in `lib.rs`: owns the HUD / scroll state and
 * emits the same Tauri events (`dial:rotate`, `dial:click`, `hud:visibility`, ...) to the React UI,
 * which keeps the wheel logic exactly as on desktop. Gesture model: CLAUDE.md §5.1. Everything
 * runs on the main thread.
 */
object DialController {
    private val main = Handler(Looper.getMainLooper())

    var a11y: DialA11yService? = null
    var session: OwnSession? = null
    var volume: VolumeWatcher? = null

    /** Latest config saved by the React UI (`save_config`); used for context matching. */
    var config: JSONObject? = null
    private var foregroundPackage: String? = null

    var isHudOpen = false
        private set
    var isScrollMode = false
        private set

    private var lastClickMs = 0L
    private var lastGestureMs = 0L

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
            ClickAction.EXIT_SCROLL -> stopScrollMode()
            ClickAction.PLAY_PAUSE -> forward(PlayerForwarder.Command.PLAY_PAUSE)
            ClickAction.CONFIRM -> WebUi.emit("dial:click")
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
            GestureAction.EXIT_SCROLL -> stopScrollMode()
            GestureAction.PREVIOUS_TRACK -> forward(PlayerForwarder.Command.PREVIOUS)
        }
    }

    fun onRotate(direction: Int) {
        if (isScrollMode && !isHudOpen) {
            a11y?.swipe(up = DialLogic.swipeUpFor(direction), fraction = 0.22f, durationMs = 180)
            return
        }
        if (!isHudOpen) return
        a11y?.tick()
        WebUi.emit(
            "dial:rotate",
            JSONObject().put("direction", direction).put("delta", direction).put("volume_pct", volume?.volumePct ?: 0),
        )
    }

    /** Volume mode: the lock is released and the change is real; mirror it on the gauge. */
    fun onNaturalVolume(pct: Int) {
        if (!isHudOpen) return
        a11y?.tick()
        WebUi.emit("dial:rotate", JSONObject().put("direction", 0).put("delta", 0).put("volume_pct", pct))
    }

    fun onForegroundApp(pkg: String) {
        foregroundPackage = pkg
        emitContext()
    }

    private fun emitContext() {
        val id = matchProfile(foregroundPackage)
        WebUi.emit("context:change", JSONObject().put("profile_id", id).put("app", foregroundPackage ?: ""))
    }

    private fun matchProfile(pkg: String?): String {
        val cfg = config ?: return "default"
        val profiles = cfg.optJSONObject("profiles") ?: return "default"
        val order = cfg.optJSONArray("profile_order")?.let { a -> List(a.length()) { a.getString(it) } }
            ?: profiles.keys().asSequence().toList()
        val patterns = profiles.keys().asSequence().associateWith { id ->
            val arr = profiles.getJSONObject(id).optJSONArray("match_patterns")
            if (arr == null) emptyList() else List(arr.length()) { arr.getString(it) }
        }
        return DialLogic.matchProfile(order, patterns, pkg)
    }

    /** `do_show_hud`: lock the volume, show the overlay, then tell the UI. */
    fun showHud() {
        val service = a11y
        if (service == null) {
            Diag.log("HUD needs the Dial Control accessibility service")
            toast("Enable Dial Control in Accessibility settings")
            return
        }
        if (isHudOpen) return
        isHudOpen = true
        val userVol = volume?.lock() ?: 0
        emitContext()
        service.showHud()
        WebUi.emit("hud:visibility", true)
        WebUi.emit("sync:state", JSONObject().put("volume", userVol))
        session?.takeBack("HUD opened")
        Diag.log("HUD shown (${foregroundPackage ?: "unknown app"})")
    }

    /** `do_hide_hud`: also ends scroll mode and restores the user's volume. */
    fun hideHud() {
        val wasActive = isHudOpen || isScrollMode
        isHudOpen = false
        isScrollMode = false
        volume?.unlock()
        a11y?.hideHud()
        DialService.instance?.updateNotification(null)
        if (wasActive) {
            WebUi.emit("hud:visibility", false)
            WebUi.emit("scroll_mode:exit")
            Diag.log("HUD hidden")
        }
    }

    /** `set_mode`: returns the user's real volume in %. */
    fun setMode(mode: String): Int {
        val v = volume ?: return 0
        return when (mode) {
            "volume" -> {
                isScrollMode = false
                v.release()
            }
            "scroll" -> {
                val user = v.lock()
                isScrollMode = true
                isHudOpen = true
                user
            }
            else -> {
                isScrollMode = false
                v.lock()
            }
        }
    }

    fun closeHudForScroll() {
        isHudOpen = false
        isScrollMode = true
        a11y?.hideHud()
        WebUi.emit("hud:visibility", false)
        DialService.instance?.updateNotification("Scroll mode: turn to scroll, click to exit")
        toast("Scroll mode: turn to scroll, click the dial to exit")
        Diag.log("scroll mode on")
    }

    fun stopScrollMode() {
        if (!isScrollMode) return
        Diag.log("scroll mode off")
        toast("Scroll mode off")
        hideHud()
    }

    /** `execute_action` for the slice types that reach the backend (modes run in the UI). */
    fun executeAction(action: JSONObject) {
        val id = action.optString("id")
        val type = action.optString("type")
        val key = action.optString("action")
        Diag.log("execute_action $id ($type)")
        hideHud()
        val service = a11y
        // Let the overlay disappear first so gestures and global actions hit the app behind it.
        main.postDelayed({
            when {
                type == "close" -> {}
                type == "mute" || id == "mute_toggle" -> Actions.toggleMute(appContext() ?: return@postDelayed)
                key == "playpause" -> forward(PlayerForwarder.Command.PLAY_PAUSE)
                key == "next" -> forward(PlayerForwarder.Command.NEXT)
                key == "prev" -> forward(PlayerForwarder.Command.PREVIOUS)
                type == "launch" -> service?.let { Actions.launch(it, action.optString("target")) }
                type == "system" -> service?.let { Actions.runSystem(it, key) }
                else -> toast("This slice type needs the desktop app")
            }
        }, 150)
    }

    private fun forward(cmd: PlayerForwarder.Command) {
        val ctx = appContext() ?: return
        if (!PlayerForwarder.forward(ctx, cmd)) toast("Grant notification access to control your player")
    }

    private fun appContext() = (a11y ?: DialService.instance)?.applicationContext

    private fun toast(msg: String) {
        appContext()?.let { Toast.makeText(it, msg, Toast.LENGTH_SHORT).show() }
    }
}
