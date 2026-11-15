package com.yuvraj.dialcontrol.web

import android.content.Context
import android.content.Intent
import com.yuvraj.dialcontrol.DialController
import com.yuvraj.dialcontrol.Diag
import com.yuvraj.dialcontrol.actions.Actions
import org.json.JSONArray
import org.json.JSONObject
import java.io.File

/**
 * The Tauri commands the React UI invokes (`src-tauri/src/lib.rs`), implemented for Android.
 * Each returns a JSON literal. Desktop-only commands (global shortcut registration) are no-ops.
 */
object Commands {
    fun handle(context: Context, cmd: String, args: JSONObject): String = when (cmd) {
        "load_config" -> loadConfig(context)?.toString() ?: "null"
        "save_config" -> {
            val config = args.getJSONObject("config")
            File(context.filesDir, CONFIG_FILE).writeText(config.toString())
            DialController.config = config
            WebUi.emit("config:update", config)
            "true"
        }
        "set_mode" -> DialController.setMode(args.optString("mode")).toString()
        "hide_hud" -> { DialController.hideHud(); "null" }
        "show_hud" -> { DialController.showHud(); "null" }
        "toggle_hud" -> { DialController.toggleHud(); "null" }
        "close_hud_for_scroll" -> { DialController.closeHudForScroll(); "null" }
        "stop_scroll_mode" -> { DialController.stopScrollMode(); "null" }
        "execute_action" -> { DialController.executeAction(args.getJSONObject("action")); "null" }
        "get_system_volume" -> (DialController.volume?.volumePct ?: 0).toString()
        "set_system_volume" -> {
            DialController.volume?.setPct(args.optInt("pct"))
            args.optInt("pct").toString()
        }
        "get_system_brightness" -> Actions.brightnessPct(context).toString()
        "set_system_brightness" -> Actions.setBrightness(context, args.optInt("pct")).toString()
        "step_brightness" -> { Actions.stepBrightness(context, args.optInt("direction")); "null" }
        "adjust_gauge" -> {
            val delta = args.optInt("delta")
            when (args.optString("mode")) {
                "scroll" -> DialController.onRotate(if (delta > 0) 1 else -1)
                "brightness" -> Actions.stepBrightness(context, delta)
                "volume" -> DialController.volume?.let { it.setPct(it.volumePct + delta) }
            }
            "null"
        }
        "list_installed_apps" -> installedApps(context).toString()
        "diag_log" -> { Diag.log("ui: ${args.optString("msg")}"); "null" }
        "show_settings" -> {
            context.startActivity(
                Intent(context, com.yuvraj.dialcontrol.SettingsActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK),
            )
            "null"
        }
        // Desktop global shortcuts have no Android equivalent.
        "register_activation_shortcut" -> JSONObject.quote("")
        "set_recording_shortcut" -> "null"
        else -> {
            Diag.log("unknown command $cmd")
            "null"
        }
    }

    private const val CONFIG_FILE = "config.json"

    fun loadConfig(context: Context): JSONObject? =
        runCatching { JSONObject(File(context.filesDir, CONFIG_FILE).readText()) }.getOrNull()

    private fun installedApps(context: Context): JSONArray {
        val pm = context.packageManager
        val launcher = Intent(Intent.ACTION_MAIN).addCategory(Intent.CATEGORY_LAUNCHER)
        val apps = pm.queryIntentActivities(launcher, 0)
            .map { it.loadLabel(pm).toString() to it.activityInfo.packageName }
            .filter { it.second != context.packageName }
            .distinctBy { it.second }
            .sortedBy { it.first.lowercase() }
        return JSONArray().apply {
            apps.forEach { (name, pkg) -> put(JSONObject().put("name", name).put("target", pkg)) }
        }
    }
}
