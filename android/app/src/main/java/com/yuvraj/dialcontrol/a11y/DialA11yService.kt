package com.yuvraj.dialcontrol.a11y

import android.accessibilityservice.AccessibilityService
import android.accessibilityservice.GestureDescription
import android.graphics.Path
import android.graphics.PixelFormat
import android.view.Gravity
import android.view.WindowManager
import android.view.accessibility.AccessibilityEvent
import com.yuvraj.dialcontrol.DialController
import com.yuvraj.dialcontrol.DialService
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.hud.WheelView

/**
 * Hosts everything that needs to act on other apps: the wheel overlay, the foreground app for
 * profile matching, scroll gestures and global actions (back, home, screenshot...).
 */
class DialA11yService : AccessibilityService() {
    private var wheel: WheelView? = null
    private val wm by lazy { getSystemService(WindowManager::class.java) }

    override fun onServiceConnected() {
        DialController.a11y = this
        Diag.log("accessibility service connected")
        if (DialService.instance == null) runCatching { DialService.start(this) }
            .onFailure { Diag.log("could not start dial service: ${it.message}") }
    }

    override fun onAccessibilityEvent(event: AccessibilityEvent) {
        if (event.eventType != AccessibilityEvent.TYPE_WINDOW_STATE_CHANGED) return
        val pkg = event.packageName?.toString() ?: return
        // Our overlay, the shade and keyboards are not the app the user is working in.
        if (pkg == packageName || pkg == "com.android.systemui" || pkg.contains("inputmethod")) return
        if (pkg != DialController.foregroundPackage) {
            DialController.foregroundPackage = pkg
            // Like desktop: a context change while the HUD is open waits for the next opening.
            if (!DialController.isHudOpen) Diag.log("foreground app: $pkg")
        }
    }

    override fun onInterrupt() {}

    override fun onDestroy() {
        hideWheel()
        if (DialController.a11y === this) DialController.a11y = null
        super.onDestroy()
    }

    fun showWheel() {
        val view = wheel ?: WheelView(this).also { wheel = it }
        if (view.parent == null) {
            val size = (resources.displayMetrics.density * 320).toInt()
            val lp = WindowManager.LayoutParams(
                size, size,
                WindowManager.LayoutParams.TYPE_ACCESSIBILITY_OVERLAY,
                WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE or
                    WindowManager.LayoutParams.FLAG_NOT_TOUCH_MODAL or
                    WindowManager.LayoutParams.FLAG_WATCH_OUTSIDE_TOUCH,
                PixelFormat.TRANSLUCENT,
            ).apply { gravity = Gravity.CENTER }
            wm.addView(view, lp)
        }
        view.refresh(tick = false)
    }

    fun hideWheel() {
        wheel?.let { if (it.parent != null) runCatching { wm.removeView(it) } }
    }

    fun refreshWheel(tick: Boolean) {
        wheel?.refresh(tick)
    }

    /** One vertical swipe through the middle of the screen; [up] scrolls content down. */
    fun swipe(up: Boolean, fraction: Float, durationMs: Long) {
        val m = resources.displayMetrics
        val x = m.widthPixels / 2f
        val mid = m.heightPixels / 2f
        val half = m.heightPixels * fraction / 2f
        val (from, to) = if (up) (mid + half) to (mid - half) else (mid - half) to (mid + half)
        stroke(x, from, x, to, durationMs)
    }

    fun swipeHorizontal(left: Boolean, fraction: Float, durationMs: Long) {
        val m = resources.displayMetrics
        val y = m.heightPixels / 2f
        val mid = m.widthPixels / 2f
        val half = m.widthPixels * fraction / 2f
        val (from, to) = if (left) (mid + half) to (mid - half) else (mid - half) to (mid + half)
        stroke(from, y, to, y, durationMs)
    }

    private fun stroke(x1: Float, y1: Float, x2: Float, y2: Float, durationMs: Long) {
        val path = Path().apply {
            moveTo(x1, y1)
            lineTo(x2, y2)
        }
        val gesture = GestureDescription.Builder()
            .addStroke(GestureDescription.StrokeDescription(path, 0, durationMs))
            .build()
        dispatchGesture(gesture, null, null)
    }
}
