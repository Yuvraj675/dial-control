package com.yuvraj.dialcontrol.profile

/**
 * Phone counterpart of `src/data/defaultProfiles.ts`. Desktop hotkeys cannot be injected on
 * Android without root, so phone slices use gestures, global actions and system toggles.
 */
enum class SliceType {
    /** `action`: scroll | volume | brightness */
    MODE,

    /** `action`: playpause | next | prev */
    MEDIA,

    /** `action`: back | home | recents | notifications | quick_settings | screenshot | lock | power | split_screen */
    GLOBAL,

    /** `action`: up | down | left | right (one swipe across the screen) */
    SWIPE,

    /** `target`: package name */
    LAUNCH,
    TORCH,
    MUTE,
    DND,
    CLOSE,
}

data class Slice(
    val id: String,
    val label: String,
    val type: SliceType,
    val action: String = "",
    val target: String = "",
)

data class Profile(
    val id: String,
    val name: String,
    /** Package names; an empty list marks the fallback profile. */
    val packages: List<String>,
    val slices: List<Slice>,
)

object Profiles {
    private val close = Slice("close", "CLOSE", SliceType.CLOSE)
    private val scroll = Slice("scroll_mode", "SCROLL", SliceType.MODE, "scroll")
    private val volume = Slice("volume_mode", "VOLUME", SliceType.MODE, "volume")
    private val brightness = Slice("brightness_mode", "BRIGHTNESS", SliceType.MODE, "brightness")
    private val playPause = Slice("media_playpause", "PLAY / PAUSE", SliceType.MEDIA, "playpause")
    private val back = Slice("back", "BACK", SliceType.GLOBAL, "back")

    val defaults: List<Profile> = listOf(
        Profile(
            "default", "System", emptyList(),
            listOf(
                playPause,
                brightness,
                volume,
                scroll,
                Slice("media_next", "NEXT TRACK", SliceType.MEDIA, "next"),
                Slice("torch", "TORCH", SliceType.TORCH),
                Slice("notifications", "NOTIFICATIONS", SliceType.GLOBAL, "notifications"),
                Slice("recents", "RECENTS", SliceType.GLOBAL, "recents"),
                Slice("screenshot", "SCREENSHOT", SliceType.GLOBAL, "screenshot"),
                close,
            ),
        ),
        Profile(
            "video", "Video",
            listOf(
                "com.google.android.youtube", "app.revanced.android.youtube", "com.netflix.mediaclient",
                "com.amazon.avod.thirdpartyclient", "org.videolan.vlc", "com.mxtech.videoplayer.ad",
                "com.jio.media.ondemand", "in.startv.hotstar",
            ),
            listOf(
                playPause,
                scroll,
                Slice("next_video", "NEXT SHORT", SliceType.SWIPE, "up"),
                volume,
                brightness,
                back,
                close,
            ),
        ),
        Profile(
            "feed", "Feeds",
            listOf(
                "com.instagram.android", "com.zhiliaoapp.musically", "com.reddit.frontpage",
                "com.twitter.android", "com.facebook.katana", "com.linkedin.android",
            ),
            listOf(
                scroll,
                Slice("next_post", "NEXT", SliceType.SWIPE, "up"),
                Slice("prev_post", "PREVIOUS", SliceType.SWIPE, "down"),
                volume,
                back,
                close,
            ),
        ),
        Profile(
            "music", "Music",
            listOf("com.spotify.music", "com.google.android.apps.youtube.music", "com.apple.android.music", "com.jio.media.jiobeats"),
            listOf(
                playPause,
                Slice("media_next", "NEXT TRACK", SliceType.MEDIA, "next"),
                Slice("media_prev", "PREV TRACK", SliceType.MEDIA, "prev"),
                volume,
                Slice("mute", "MUTE", SliceType.MUTE),
                close,
            ),
        ),
    )

    /** Exact package match wins; otherwise the fallback (empty package list) profile. */
    fun match(profiles: List<Profile>, packageName: String?): Profile {
        val hit = packageName?.let { pkg -> profiles.firstOrNull { pkg in it.packages } }
        return hit ?: profiles.firstOrNull { it.packages.isEmpty() } ?: profiles.first()
    }
}
