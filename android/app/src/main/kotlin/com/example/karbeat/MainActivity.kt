package com.example.karbeat

import io.flutter.embedding.android.FlutterActivity
import android.os.Bundle

class MainActivity : FlutterActivity() {
    // Declare the native method we defined in Rust
    external fun initRust(context: android.content.Context)

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        System.loadLibrary("karbeat_flutter_ffi")

        // Initialize the ndk-context!
        initRust(this.applicationContext)
    }
}
