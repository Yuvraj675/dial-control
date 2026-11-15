package com.yuvraj.dialcontrol.web

import android.annotation.SuppressLint
import android.content.Context
import android.graphics.Color
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.webkit.JavascriptInterface
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebView
import android.webkit.WebViewClient
import com.yuvraj.dialcontrol.Diag
import org.json.JSONObject
import java.io.ByteArrayInputStream

/**
 * Runs the desktop React UI (the Vite bundle copied into `assets/web`) in a WebView.
 *
 * The bundle is served from a fake https origin so ES modules and localStorage work, and
 * `index.html` gets `shim/shim.js` injected first: it implements `window.__TAURI_INTERNALS__`,
 * so `invoke` lands in [Commands] and Kotlin emits Tauri events with [emit].
 */
object WebUi {
    private const val HOST = "appassets.androidplatform.net"
    private val main = Handler(Looper.getMainLooper())
    private val views = mutableSetOf<WebView>()

    /** [designWidth]: CSS width the page is laid out at, scaled to fit the WebView. */
    fun url(view: String, designWidth: Int) = "https://$HOST/index.html?view=$view&fit=$designWidth"

    @SuppressLint("SetJavaScriptEnabled")
    fun create(context: Context, transparent: Boolean): WebView {
        val web = WebView(context)
        web.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            useWideViewPort = true
            loadWithOverviewMode = true
            setSupportZoom(false)
            builtInZoomControls = false
            mediaPlaybackRequiresUserGesture = false
        }
        if (transparent) web.setBackgroundColor(Color.TRANSPARENT)
        web.isVerticalScrollBarEnabled = false
        web.isHorizontalScrollBarEnabled = false
        web.addJavascriptInterface(Bridge(web), "DialBridge")
        web.webViewClient = object : WebViewClient() {
            override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? =
                serve(context, request.url)
        }
        views += web
        return web
    }

    fun destroy(web: WebView) {
        views -= web
        web.destroy()
    }

    /** Tauri `Emitter::emit`: every WebView receives the event (exactly once each). */
    fun emit(event: String, payload: Any? = null) {
        val json = when (payload) {
            null -> "null"
            is JSONObject -> payload.toString()
            is String -> JSONObject.quote(payload)
            else -> payload.toString()
        }
        val js = "window.__DC_EMIT__ && window.__DC_EMIT__(${JSONObject.quote(event)}, $json)"
        main.post { views.forEach { it.evaluateJavascript(js, null) } }
    }

    private fun serve(context: Context, url: Uri): WebResourceResponse? {
        if (url.host != HOST) return null
        val path = url.path?.trimStart('/').orEmpty().ifEmpty { "index.html" }
        return runCatching {
            if (path == "__shim.js") return response("application/javascript", context.assets.open("shim/shim.js").readBytes())
            val bytes = context.assets.open("web/$path").readBytes()
            if (path == "index.html") {
                val fit = url.getQueryParameter("fit") ?: "500"
                val html = String(bytes)
                    .replace(Regex("<meta name=\"viewport\"[^>]*>"), "<meta name=\"viewport\" content=\"width=$fit, user-scalable=no\" />")
                    .replace("<head>", "<head><script src=\"/__shim.js\"></script>")
                response("text/html", html.toByteArray())
            } else {
                response(mime(path), bytes)
            }
        }.getOrElse { WebResourceResponse("text/plain", "utf-8", 404, "Not Found", emptyMap(), ByteArrayInputStream(ByteArray(0))) }
    }

    private fun response(mime: String, bytes: ByteArray) =
        WebResourceResponse(mime, "utf-8", ByteArrayInputStream(bytes))

    private fun mime(path: String) = when (path.substringAfterLast('.').lowercase()) {
        "js", "mjs" -> "application/javascript"
        "css" -> "text/css"
        "html" -> "text/html"
        "json" -> "application/json"
        "svg" -> "image/svg+xml"
        "png" -> "image/png"
        "jpg", "jpeg" -> "image/jpeg"
        "webp" -> "image/webp"
        "woff2" -> "font/woff2"
        "woff" -> "font/woff"
        "otf" -> "font/otf"
        "ttf" -> "font/ttf"
        else -> "application/octet-stream"
    }

    private class Bridge(private val web: WebView) {
        @JavascriptInterface
        fun invoke(cmd: String, argsJson: String, requestId: Int) {
            main.post {
                val (ok, value) = runCatching {
                    true to Commands.handle(web.context, cmd, JSONObject(argsJson.ifEmpty { "{}" }))
                }.getOrElse {
                    Diag.log("command $cmd failed: ${it.message}")
                    false to JSONObject.quote(it.message ?: "error")
                }
                web.evaluateJavascript("window.__DC_RESOLVE__($requestId, $ok, $value)", null)
            }
        }
    }
}
