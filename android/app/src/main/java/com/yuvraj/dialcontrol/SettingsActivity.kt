package com.yuvraj.dialcontrol

import android.app.Activity
import android.graphics.Color
import android.os.Bundle
import android.webkit.WebView
import com.yuvraj.dialcontrol.web.WebUi

/**
 * The desktop Settings window (profiles, slices, appearance), unchanged, in a WebView. It is
 * laid out at the desktop's 1080 CSS px width and scaled to fit, so the screen is landscape.
 */
class SettingsActivity : Activity() {
    private var web: WebView? = null

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.statusBarColor = Color.BLACK
        window.navigationBarColor = Color.BLACK
        val view = WebUi.create(this, transparent = false).apply { setBackgroundColor(Color.parseColor("#07080A")) }
        setContentView(view)
        view.loadUrl(WebUi.url("settings", 1080))
        web = view
    }

    override fun onDestroy() {
        web?.let { WebUi.destroy(it) }
        web = null
        super.onDestroy()
    }
}
