plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

android {
    namespace = "dev.peapod.android"
    compileSdk = 34
    ndkVersion = "25.2.9519653"
    defaultConfig {
        applicationId = "dev.peapod.android"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"
        externalNativeBuild {
            cmake {
                cppFlags += ""
            }
        }
    }
    signingConfigs {
        create("release") {
            val storeFileEnv = project.findProperty("RELEASE_STORE_FILE")?.toString()
            val storePwd = project.findProperty("RELEASE_STORE_PASSWORD")?.toString()
            val keyAliasEnv = project.findProperty("RELEASE_KEY_ALIAS")?.toString()
            val keyPwd = project.findProperty("RELEASE_KEY_PASSWORD")?.toString()
            if (storeFileEnv != null && storePwd != null && keyAliasEnv != null && keyPwd != null) {
                storeFile = file(storeFileEnv)
                storePassword = storePwd
                keyAlias = keyAliasEnv
                keyPassword = keyPwd
            }
        }
    }
    buildTypes {
        release {
            isMinifyEnabled = false
            val releaseConfig = signingConfigs.findByName("release")
            if (releaseConfig != null && releaseConfig.storeFile?.exists() == true) {
                signingConfig = releaseConfig
            }
        }
    }
    buildFeatures {
        viewBinding = true
    }
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
}

dependencies {
    implementation("androidx.activity:activity-ktx:1.8.2")
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
}
