package com.yuvraj.dialcontrol.hud

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.RectF
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import com.yuvraj.dialcontrol.DialController
import com.yuvraj.dialcontrol.DialController.Mode
import com.yuvraj.dialcontrol.R
import kotlin.math.atan2
import kotlin.math.cos
import kotlin.math.hypot
import kotlin.math.min
import kotlin.math.sin

/**
 * The radial wheel, drawn like the desktop DonutRadialHUD: one donut segment per slice with the
 * highlighted slice in Nothing red, the selected label in the centre, or a gauge in
 * volume / brightness mode. Reads its state straight from [DialController].
 */
@SuppressLint("ViewConstructor")
class WheelView(context: Context) : View(context) {
    private val density = resources.displayMetrics.density
    private val red = Color.parseColor("#D71921")
    private val dot = runCatching { resources.getFont(R.font.ndot57) }.getOrNull()
    private val body = runCatching { resources.getFont(R.font.ntype82) }.getOrNull()

    private val backdrop = Paint(Paint.ANTI_ALIAS_FLAG).apply { color = Color.argb(235, 10, 10, 10) }
    private val segment = Paint(Paint.ANTI_ALIAS_FLAG).apply { style = Paint.Style.STROKE }
    private val label = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textAlign = Paint.Align.CENTER
        typeface = body
    }
    private val center = Paint(Paint.ANTI_ALIAS_FLAG).apply {
        color = Color.WHITE
        textAlign = Paint.Align.CENTER
        typeface = dot
    }
    private val arc = RectF()

    fun refresh(tick: Boolean) {
        if (tick) performHapticFeedback(HapticFeedbackConstants.CLOCK_TICK)
        invalidate()
    }

    override fun onDraw(canvas: Canvas) {
        val cx = width / 2f
        val cy = height / 2f
        val outer = min(cx, cy) - 4 * density
        val thickness = 56 * density
        val ring = outer - thickness / 2
        canvas.drawCircle(cx, cy, outer, backdrop)
        arc.set(cx - ring, cy - ring, cx + ring, cy + ring)
        segment.strokeWidth = thickness - 6 * density

        when (DialController.mode) {
            Mode.MENU -> drawMenu(canvas, cx, cy, ring)
            Mode.VOLUME -> drawGauge(canvas, cx, cy, "VOLUME")
            Mode.BRIGHTNESS -> drawGauge(canvas, cx, cy, "BRIGHTNESS")
        }
    }

    private fun drawMenu(canvas: Canvas, cx: Float, cy: Float, ring: Float) {
        val slices = DialController.profile.slices
        val n = slices.size.coerceAtLeast(1)
        val sweep = 360f / n
        val gap = 2.5f
        label.textSize = 9 * density
        slices.forEachIndexed { i, s ->
            val start = -90f + i * sweep - sweep / 2
            val sel = i == DialController.selected
            segment.color = if (sel) red else Color.argb(255, 38, 38, 38)
            canvas.drawArc(arc, start + gap / 2, sweep - gap, false, segment)
            val mid = Math.toRadians((start + sweep / 2).toDouble())
            label.color = if (sel) Color.WHITE else Color.argb(200, 220, 220, 220)
            val words = s.label.split(" / ", " ")
            val lx = cx + ring * cos(mid).toFloat()
            var ly = cy + ring * sin(mid).toFloat() - (words.size - 1) * label.textSize / 2 + label.textSize / 3
            for (w in words.take(2)) {
                canvas.drawText(w, lx, ly, label)
                ly += label.textSize
            }
        }
        val selected = slices.getOrNull(DialController.selected)?.label ?: ""
        center.textSize = 20 * density
        center.color = Color.WHITE
        canvas.drawText(selected, cx, cy + 4 * density, center)
        label.textSize = 10 * density
        label.color = red
        canvas.drawText(DialController.profile.name.uppercase(), cx, cy + 24 * density, label)
    }

    private fun drawGauge(canvas: Canvas, cx: Float, cy: Float, title: String) {
        val pct = DialController.gaugePct
        segment.color = Color.argb(255, 38, 38, 38)
        canvas.drawArc(arc, 135f, 270f, false, segment)
        segment.color = red
        canvas.drawArc(arc, 135f, 270f * pct / 100f, false, segment)
        center.textSize = 44 * density
        center.color = Color.WHITE
        canvas.drawText("$pct", cx, cy + 14 * density, center)
        label.textSize = 10 * density
        label.color = red
        canvas.drawText(title, cx, cy + 36 * density, label)
    }

    @SuppressLint("ClickableViewAccessibility")
    override fun onTouchEvent(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_OUTSIDE -> DialController.hideHud()
            MotionEvent.ACTION_UP -> {
                val dx = event.x - width / 2f
                val dy = event.y - height / 2f
                val inner = min(width, height) / 2f - 60 * density
                if (DialController.mode != Mode.MENU || hypot(dx, dy) < inner) {
                    DialController.hideHud()
                } else {
                    val n = DialController.profile.slices.size
                    // Angle from 12 o'clock, clockwise, offset by half a slice (slice 0 is centred at the top).
                    val deg = (Math.toDegrees(atan2(dy, dx).toDouble()) + 90 + 360 + 180.0 / n) % 360
                    DialController.selectAndRun((deg / (360.0 / n)).toInt())
                }
            }
        }
        return true
    }
}
