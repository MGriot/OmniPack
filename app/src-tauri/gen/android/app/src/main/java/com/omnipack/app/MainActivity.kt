package com.omnipack.app

import android.graphics.Color
import android.os.Bundle
import android.view.View
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // Dark bars with light icons, matching the app theme.
    val bar = Color.parseColor("#181b22")
    enableEdgeToEdge(SystemBarStyle.dark(bar), SystemBarStyle.dark(bar))
    super.onCreate(savedInstanceState)
    // Keep the web UI clear of the status bar, navigation bar and cutouts.
    val root = findViewById<View>(android.R.id.content)
    root.setBackgroundColor(bar)
    ViewCompat.setOnApplyWindowInsetsListener(root) { v, insets ->
      val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      v.setPadding(bars.left, bars.top, bars.right, bars.bottom)
      WindowInsetsCompat.CONSUMED
    }
  }
}
