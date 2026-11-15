package com.yuvraj.dialcontrol.a11y

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.graphics.PixelFormat
import android.os.Handler
import android.os.Looper
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.View
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import android.webkit.WebView
import com.yuvraj.dialcontrol.DialController
import com.yuvraj.dialcontrol.DialService
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.web.WebUi

/**
 * Hosts everything that acts on other apps: the HUD overlay (the desktop React wheel in a
 * transparent WebView, the Android counterpart of the `hud` Tauri window), the foreground app
 * for profile matching, scroll gestures and global actions.
 *
 * The overlay stays attached and preloaded so it appears instantly; while hidden it is
 * invisible and not touchable.
 */
class DialA11yService : AccessibilityService() {
    private var hud: WebView? = null
    private var hudParams: WindowManager.LayoutParams? = null
    private val wm by lazy { getSystemService(WindowManager::class.java) }
    private val main = Handler(Looper.getMainLooper())
    private val hideAfterAnimation = Runnable { setHudShown(false) }

    override fun onServiceConnected() {
        DialController.a11y = this
        Diag.log("accessibility service connected")
        if (DialService.instance == null) runCatching { DialService.start(this) }
            .onFailure { Diag.log("could not start dial service: ${it.message}") }
        createHud()
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString() ?: return
        // Our overlay, the shade and keyboards are not the app the user is working in.
        if (pkg == packageName || pkg == "com.android.systemui" || pkg.contains("inputmethod")) return
        DialController.onForegroundApp(pkg)
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        hud?.let {
            runCatching { wm.removeView(it) }
            WebUi.destroy(it)
        }
        hud = null
        if (DialController.a11y === this) DialController.a11y = null
        super.onDestroy()
    }

    /** Desktop HUD window is 500x500; the page is laid out 500 CSS px wide and scaled to fit. */
    private fun createHud() {
        if (hud != null) return
        val m = resources.displayMetrics
        val side = (minOf(m.widthPixels, m.heightPixels) * 0.96f).toInt()
        val web = WebUi.create(this, transparent = true)
        web.visibility = View.INVISIBLE
        val lp = WindowManager.LayoutParams(
            side, side,
            WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
            HIDDEN_FLAGS,
            PixelFormat.TRANSLUCENT,
        ).apply { gravity = Gravity.CENTER }
        wm.addView(web, lp)
        web.loadUrl(WebUi.url("hud", 500))
        hud = web
        hudParams = lp
    }

    fun showHud() {
        createHud()
        main.removeCallbacks(hideAfterAnimation)
        setHudShown(true)
    }

    /** The UI plays its close animation after `hud:visibility false`; then the window goes away. */
    fun hideHud() {
        main.removeCallbacks(hideAfterAnimation)
        main.postDelayed(hideAfterAnimation, 220)
    }

    private fun setHudShown(shown: Boolean) {
        val web = hud ?: return
        val lp = hudParams ?: return
        lp.flags = if (shown) SHOWN_FLAGS else HIDDEN_FLAGS
        web.visibility = if (shown) View.VISIBLE else View.INVISIBLE
        runCatching { wm.updateViewLayout(web, lp) }
    }

    /** Detent feedback while the HUD is open. */
    fun tick() {
        hud?.performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
    }

    /** One vertical swipe through the middle of the screen; [up] scrolls content down. */
    fun swipe(up: Boolean, fraction: Float, durationMs: Long) {
        val m = resources.displayMetrics
        val x = m.widthPixels / 2f
        val mid = m.heightPixels / 2f
        val half = m.heightPixels * fraction / 2f
        val (from, to) = if (up) (mid + half) to (mid - half) else (mid - half) to (mid + half)
        val path = Path().apply {
            moveTo(x, from)
            lineTo(x, to)
        }
        dispatchGesture(
            GestureDescription.Builder().addStroke(GestureDescription.StrokeDescription(path, 0, durationMs)).build(),
            null,
            null,
        )
    }

    private companion object {
        const val SHOWN_FLAGS = WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
            WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or
            WindowManager.LayoutParams.FLAG_LAYOUT_IN_SCREEN
        const val HIDDEN_FLAGS = SHOWN_FLAGS or WindowManager.LayoutParams.FLAG_NOT_TOUCHABLE
    }
}
