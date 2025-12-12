// File: android/app/build.gradle.kts

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

android {
    namespace = "com.cfait"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.cfait"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = "0.3.0"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDir("src/main/jniLibs")
        }
    }

    buildFeatures {
        compose = true
    }
    
    composeOptions {
    }

    // --- ADD/UPDATE THIS SECTION ---
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_21
        targetCompatibility = JavaVersion.VERSION_21
    }

    kotlinOptions {
        jvmTarget = "21"
    }
    // -------------------------------
}

dependencies {
    implementation("androidx.core:core-ktx:1.17.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.10.0")
    implementation("androidx.activity:activity-compose:1.12.1")
    implementation(platform("androidx.compose:compose-bom:2025.12.00"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")
    implementation("androidx.navigation:navigation-compose:2.9.6")
    
    // Required for UniFFI
    implementation("net.java.dev.jna:jna:5.18.1@aar")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
}

tasks.register<Copy>("copyFonts") {
    description = "Copies fonts from root assets to Android resources"
    // Go up two levels from android/app/ to root
    from("${project.rootDir}/../assets/fonts/SymbolsNerdFont-Regular.ttf")
    into("${project.projectDir}/src/main/res/font")
    // Android requires lowercase snake_case for resource files
    rename { "symbols_nerd_font.ttf" }
}

// Hook into the build process so it happens automatically
tasks.named("preBuild") {
    dependsOn("copyFonts")
}