package com.yuvraj.dialcontrol

import android.os.Handler
import android.os.Looper
import android.util.Log
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

/**
 * Field diagnostics, the Android counterpart of `platform::diag`: every dial signal goes to
 * logcat (`adb logcat -s DialControl`) and to a ring buffer shown live in the main screen.
 */
object Diag {
    private const val TAG = "DialControl"
    private const val MAX_LINES = 200
    private val lines = ArrayDeque<String>()
    private val listeners = mutableSetOf<() -> Unit>()
    private val main = Handler(Looper.getMainLooper())
    private val time = SimpleDateFormat("HH:mm:ss.SSS", Locale.US)

    fun log(msg: String) {
        Log.i(TAG, msg)
        val line = "${time.format(Date())}  $msg"
        synchronized(lines) {
            lines.addLast(line)
            while (lines.size > MAX_LINES) lines.removeFirst()
        }
        main.post { listeners.toList().forEach { it() } }
    }

    fun snapshot(): List<String> = synchronized(lines) { lines.toList() }

    fun addListener(l: () -> Unit) {
        listeners += l
    }

    fun removeListener(l: () -> Unit) {
        listeners -= l
    }
}
