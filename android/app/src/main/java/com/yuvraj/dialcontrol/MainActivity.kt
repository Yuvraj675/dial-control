package com.yuvraj.dialcontrol

import android.Manifest
import android.app.Activity
import android.app.NotificationManager
import android.content.ClipData
import android.content.ClipboardManager
import android.content.ComponentName
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.Settings
import android.view.Gravity
import android.view.View
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import android.widget.Toast
import com.yuvraj.dialcontrol.media.MediaListener

/**
 * Setup screen, styled like the desktop Settings window (#07080A background, #0C0E14 cards,
 * Ndot uppercase labels, Nothing red). Permission checklist, listener control, a button into the
 * desktop Settings UI, and a live log of every dial signal.
 */
class MainActivity : Activity() {
    private val red = Color.parseColor("#E2201F")
    private val bg = Color.parseColor("#07080A")
    private val card = Color.parseColor("#0C0E14")
    private val border = Color.argb(26, 255, 255, 255)
    private val muted = Color.parseColor("#A3A3A3")
    private val dim = Color.parseColor("#737373")
    private val ndot by lazy { resources.getFont(R.font.ndot57) }
    private val body by lazy { resources.getFont(R.font.ntype82) }

    private lateinit var checklist: LinearLayout
    private lateinit var serviceButton: TextView
    private lateinit var status: TextView
    private lateinit var log: TextView
    private val onLog: () -> Unit = { renderLog() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = bg
        window.navigationBarColor = bg
        val pad = dp(20)
        val root = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(pad, dp(36), pad, pad)
        }
        root.addView(text("DIAL CONTROL", 30f, Color.WHITE, ndot))
        status = text("", 12f, muted, ndot).apply { setPadding(0, dp(6), 0, 0) }
        root.addView(status)

        root.addView(primaryButton("CUSTOMIZE WHEEL") { startActivity(Intent(this, SettingsActivity::class.java)) })
        root.addView(outlineButton("OPEN WHEEL NOW") { DialController.showHud() })
        serviceButton = outlineButton("") {
            if (DialService.instance == null) DialService.start(this) else DialService.stop(this)
            serviceButton.postDelayed({ render() }, 300)
        }
        root.addView(serviceButton)

        root.addView(section("SETUP"))
        checklist = cardLayout()
        root.addView(checklist)

        root.addView(section("GESTURES"))
        root.addView(cardLayout().apply {
            for ((g, what) in listOf(
                "TURN" to "Volume  ·  moves the highlight when the wheel is open",
                "CLICK" to "Play / pause  ·  runs the highlighted slice",
                "DOUBLE PRESS" to "Open the wheel  ·  dismiss it",
                "TRIPLE PRESS" to "Previous track  ·  dismiss the wheel",
            )) {
                addView(row(g, what, null))
            }
        })

        val logHeader = LinearLayout(this).apply {
            orientation = LinearLayout.HORIZONTAL
            gravity = Gravity.CENTER_VERTICAL
            addView(section("DIAL LOG"), LinearLayout.LayoutParams(0, LinearLayout.LayoutParams.WRAP_CONTENT, 1f))
            addView(text("COPY", 11f, red, ndot).apply {
                setPadding(dp(12), dp(24), 0, dp(8))
                setOnClickListener {
                    getSystemService(ClipboardManager::class.java)
                        .setPrimaryClip(ClipData.newPlainText("Dial Control log", Diag.snapshot().joinToString("\n")))
                    Toast.makeText(this@MainActivity, "Log copied", Toast.LENGTH_SHORT).show()
                }
            })
        }
        root.addView(logHeader)
        log = text("", 10.5f, dim, Typeface.MONOSPACE)
        root.addView(cardLayout().apply { addView(log) })

        setContentView(ScrollView(this).apply {
            setBackgroundColor(bg)
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
        val a11y = DialController.a11y != null
        val notif = hasNotificationAccess()
        step("ACCESSIBILITY SERVICE", "Shows the wheel over other apps, scrolls, runs phone actions", a11y) {
            startActivity(Intent(Settings.ACTION_ACCESSIBILITY_SETTINGS))
        }
        step("NOTIFICATION ACCESS", "Lets a click play / pause your real player", notif) {
            startActivity(Intent(Settings.ACTION_NOTIFICATION_LISTENER_SETTINGS))
        }
        step("MODIFY SYSTEM SETTINGS", "Brightness gauge", Settings.System.canWrite(this)) {
            startActivity(Intent(Settings.ACTION_MANAGE_WRITE_SETTINGS, Uri.parse("package:$packageName")))
        }
        step(
            "DO NOT DISTURB ACCESS",
            "Optional, for DND slices",
            getSystemService(NotificationManager::class.java).isNotificationPolicyAccessGranted,
        ) { startActivity(Intent(Settings.ACTION_NOTIFICATION_POLICY_ACCESS_SETTINGS)) }

        val running = DialService.instance != null
        serviceButton.text = if (running) "STOP DIAL LISTENER" else "START DIAL LISTENER"
        status.text = when {
            !running -> "LISTENER STOPPED"
            !a11y -> "ENABLE THE ACCESSIBILITY SERVICE TO SHOW THE WHEEL"
            else -> "READY  ·  DOUBLE PRESS THE DIAL"
        }
        status.setTextColor(if (running && a11y) red else muted)
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
        checklist.addView(row(title, if (done) "GRANTED" else detail, done).apply { setOnClickListener { open() } })
    }

    private fun row(title: String, detail: String, done: Boolean?): LinearLayout = LinearLayout(this).apply {
        orientation = LinearLayout.HORIZONTAL
        gravity = Gravity.CENTER_VERTICAL
        setPadding(0, dp(10), 0, dp(10))
        if (done != null) {
            addView(View(this@MainActivity).apply {
                background = GradientDrawable().apply {
                    shape = GradientDrawable.OVAL
                    if (done) setColor(red) else setStroke(dp(2), dim)
                }
            }, LinearLayout.LayoutParams(dp(10), dp(10)).apply { marginEnd = dp(14) })
        }
        addView(LinearLayout(this@MainActivity).apply {
            orientation = LinearLayout.VERTICAL
            addView(text(title, 12.5f, Color.WHITE, ndot))
            addView(text(detail, 12f, if (done == true) red else muted, body).apply { setPadding(0, dp(2), 0, 0) })
        })
    }

    private fun section(t: String) = text(t, 11f, dim, ndot).apply {
        letterSpacing = 0.15f
        setPadding(0, dp(24), 0, dp(8))
    }

    private fun cardLayout() = LinearLayout(this).apply {
        orientation = LinearLayout.VERTICAL
        setPadding(dp(16), dp(8), dp(16), dp(8))
        background = rounded(card, border)
    }

    private fun primaryButton(t: String, onClick: () -> Unit) = button(t, onClick).apply {
        setTextColor(Color.WHITE)
        background = rounded(red, red)
    }

    private fun outlineButton(t: String, onClick: () -> Unit) = button(t, onClick).apply {
        setTextColor(Color.WHITE)
        background = rounded(card, border)
    }

    private fun button(t: String, onClick: () -> Unit) = text(t, 13f, Color.WHITE, ndot).apply {
        gravity = Gravity.CENTER
        letterSpacing = 0.1f
        setPadding(0, dp(15), 0, dp(15))
        setOnClickListener { onClick() }
        layoutParams = LinearLayout.LayoutParams(
            LinearLayout.LayoutParams.MATCH_PARENT, LinearLayout.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = dp(12) }
    }

    private fun rounded(fill: Int, stroke: Int) = GradientDrawable().apply {
        cornerRadius = dp(16).toFloat()
        setColor(fill)
        setStroke(dp(1), stroke)
    }

    private fun text(t: String, size: Float, color: Int, face: Typeface? = null) = TextView(this).apply {
        text = t
        textSize = size
        setTextColor(color)
        face?.let { typeface = it }
    }

    private fun dp(v: Int) = (v * resources.displayMetrics.density).toInt()
}
