plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

val nativeDirectory = rootProject.file(providers.gradleProperty("wayformNativeDir").getOrElse("../../target/android-mobile"))
val sampleVenues = listOf("asterion-medical-center", "harbor-exchange", "meridian-park-campus", "reference-medical-centre")
val sampleAssets = tasks.register<Sync>("prepareSampleVenues") {
    into(layout.buildDirectory.dir("generated/sampleAssets"))
    from(rootProject.file("../../examples/survey/clinic-entrance.json")) { into("survey") }
    sampleVenues.forEach { venue ->
        from(rootProject.file("../../compat/voicegis/upstream/buildings/$venue/compiled/building.package.json")) {
            into("venues")
            rename { "$venue.json" }
        }
    }
    from(rootProject.file("../../compat/voicegis/upstream/README.md")) { rename { "VOICEGIS-README.md" } }
    from(rootProject.file("../../compat/voicegis/source-lock.json")) { rename { "voicegis-source-lock.json" } }
}
val checkNative = tasks.register("checkNativeBindings") {
    doLast {
        check(nativeDirectory.resolve("source-commit.txt").isFile) { "Build the native core first: bash scripts/build-android-core.sh" }
        for (abi in listOf("arm64-v8a", "x86_64")) {
            check(nativeDirectory.resolve("jniLibs/$abi/libwayform_mobile.so").isFile) { "Missing native core for $abi" }
        }
        check(nativeDirectory.resolve("kotlin").walk().any { it.extension == "kt" }) { "Missing generated Kotlin bindings" }
    }
}
android {
    namespace = "app.wayform"
    compileSdk = 36
    defaultConfig {
        applicationId = "app.wayform"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
    }
    buildFeatures { compose = true }
    compileOptions { sourceCompatibility = JavaVersion.VERSION_17; targetCompatibility = JavaVersion.VERSION_17 }
    sourceSets["main"].apply {
        java.srcDir(nativeDirectory.resolve("kotlin"))
        jniLibs.srcDir(nativeDirectory.resolve("jniLibs"))
        assets.srcDir(layout.buildDirectory.dir("generated/sampleAssets"))
    }
    buildTypes {
        release { isMinifyEnabled = false }
        create("preview") {
            initWith(getByName("release"))
            applicationIdSuffix = ".preview"
            versionNameSuffix = "-preview.1"
            signingConfig = signingConfigs.getByName("debug")
            matchingFallbacks += "release"
        }
    }
    packaging { resources.excludes += "/META-INF/{AL2.0,LGPL2.1}" }
}
kotlin { jvmToolchain(17) }
tasks.named("preBuild") { dependsOn(sampleAssets, checkNative) }

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2025.12.00")
    implementation(composeBom)
    androidTestImplementation(composeBom)
    implementation("androidx.activity:activity-compose:1.12.1")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.10.0")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.10.0")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    debugImplementation("androidx.compose.ui:ui-tooling")
    implementation("net.java.dev.jna:jna:5.18.1@aar")
    implementation("com.journeyapps:zxing-android-embedded:4.3.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
