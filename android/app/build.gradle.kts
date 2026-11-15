plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "com.yuvraj.dialcontrol"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.yuvraj.dialcontrol"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            // No release keystore yet: sign with the debug key so the APK installs.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    // The desktop React UI (`npm run build` -> ../dist) runs in the app's WebViews.
    sourceSets["main"].assets.srcDir(layout.buildDirectory.dir("generated/webui"))

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
    }
}

val webDist = rootProject.file("../dist")
val copyWebUi by tasks.registering(Copy::class) {
    description = "Copies the desktop React UI bundle into the APK assets (assets/web)."
    doFirst {
        check(webDist.resolve("index.html").exists()) { "Run `npm run build` in the repo root first (missing ${'$'}webDist)." }
    }
    from(webDist)
    into(layout.buildDirectory.dir("generated/webui/web"))
}
tasks.named("preBuild") { dependsOn(copyWebUi) }

dependencies {
    testImplementation("junit:junit:4.13.2")
}
