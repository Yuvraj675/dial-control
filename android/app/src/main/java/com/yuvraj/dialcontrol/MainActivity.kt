package com.yuvraj.dialcontrol

import android.Manifest
import android.app.Activity
import android.app.NotificationManager
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.Typeface
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.widget.Button
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import com.yuvraj.dialcontrol.media.MediaListener

/**
 * Setup screen: permission checklist, service start/stop, the active profiles and a live log of
 * every dial signal so the earbuds' behaviour on this phone can be checked at a glance.
 */
class MainActivity : Activity() {
    private val red = Color.parseColor("#D71921")
    private lateinit var checklist: LinearLayout
    private lateinit var serviceButton: Button
    private lateinit var log: TextView
    private val onLog: () -> Unit = { renderLog() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val pad = dp(20)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, pad * 2, pad, pad)
            setBackgroundColor(Color.BLACK)
        }
        root.addView(text("DIAL CONTROL", 28f, Color.WHITE, resources.getFont(R.font.ndot57)))
        root.addView(text("Double press the dial to open the wheel. Click runs the highlighted slice.", 14f, Color.LTGRAY))

        root.addView(header("SETUP"))
        checklist = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL }
        root.addView(checklist)

        serviceButton = button("") {
            if (DialService.instance == null) DialService.start(this) else DialService.stop(this)
            serviceButton.postDelayed({ render() }, 300)
        }
        root.addView(serviceButton)
        root.addView(button("OPEN WHEEL NOW") { DialController.showHud() })

        root.addView(header("PROFILES"))
        for (p in DialController.profiles) {
            val apps = if (p.packages.isEmpty()) "every other app" else "${p.packages.size} apps"
            root.addView(text("${p.name}  ·  $apps\n${p.slices.joinToString("  ·  ") { it.label }}", 13f, Color.LTGRAY))
        }

        root.addView(header("DIAL LOG"))
        log = text("", 11f, Color.GRAY, Typeface.MONOSPACE)
        root.addView(log)

        setContentView(ScrollView(this).apply {
            setBackgroundColor(Color.BLACK)
            addView(root)
        })
        if (Build.VERSION.SDK_INT >= 33 &&
            checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS) != PackageManager.PERMISSION_GRANTED
        ) {
            requestPermissions(arrayOf(Manifest.permission.POST_NOTIFICATIONS), 1)
        }
    }

    override fun onResume() {
        super.onResume()
        render()
        Diag.addListener(onLog)
    }

    override fun onPause() {
        Diag.removeListener(onLog)
        super.onPause()
    }

    private fun render() {
        checklist.removeAllViews()
        step(
            "Accessibility service",
            "Shows the wheel over other apps, scrolls and runs system actions.",
            DialController.a11y != null,
        ) { startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS)) }
        step(
            "Notification access",
            "Lets a click play/pause your real player while the wheel is closed.",
            hasNotificationAccess(),
        ) { startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS)) }
        step(
            "Modify system settings",
            "Needed for the brightness slice.",
            Settings.System.canWrite(this),
        ) {
            startActivity(Intent(Settings.ACTION_MANAGE_WRITE_SETTINGS, Uri.parse("package:$packageName")))
        }
        step(
            "Do Not Disturb access (optional)",
            "Only for DND slices.",
            getSystemService(NotificationManager::class.java).isNotificationPolicyAccessGranted,
        ) { startActivity(Intent(Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS)) }
        serviceButton.text = if (DialService.instance == null) "START DIAL LISTENER" else "STOP DIAL LISTENER"
        renderLog()
    }

    private fun renderLog() {
        log.text = Diag.snapshot().takeLast(60).reversed().joinToString("\n").ifEmpty { "Turn or press the dial…" }
    }

    private fun hasNotificationAccess(): Boolean {
        val flat = Settings.Secure.getString(contentResolver, "enabled_notification_listeners") ?: return false
        return flat.contains(ComponentName(this, MediaListener::class.java).flattenToString())
    }

    private fun step(title: String, detail: String, done: Boolean, open: () -> Unit) {
        val row = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            setPadding(0, dp(10), 0, dp(10))
            setOnClickListener { open() }
        }
        row.addView(text(if (done) "●" else "○", 18f, if (done) red else Color.GRAY).apply {
            setPadding(0, 0, dp(14), 0)
        })
        row.addView(LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            addView(text(title, 15f, Color.WHITE))
            addView(text(if (done) "Granted" else detail, 12f, Color.GRAY))
        })
        checklist.addView(row)
    }

    private fun header(t: String) = text(t, 12f, red).apply { setPadding(0, dp(24), 0, dp(6)) }

    private fun text(t: String, size: Float, color: Int, face: Typeface? = null) = TextView(this).apply {
        text = t
        textSize = size
        setTextColor(color)
        face?.let { typeface = it }
    }

    private fun button(t: String, onClick: (View) -> Unit) = Button(this).apply {
        text = t
        setTextColor(Color.WHITE)
        setBackgroundColor(Color.argb(255, 30, 30, 30))
        setOnClickListener(onClick)
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = dp(10) }
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()
}
