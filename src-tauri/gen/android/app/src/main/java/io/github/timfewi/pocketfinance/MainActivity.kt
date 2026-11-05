package io.github.timfewi.pocketfinance

import android.graphics.Color
import android.os.Bundle
import android.view.ViewGroup
import android.webkit.WebView
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.doOnAttach

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    // The receipt UI currently uses a light paper palette in both system themes.
    // Keep system icons legible, including older Android navigation bars.
    val bars = SystemBarStyle.light(Color.TRANSPARENT, Color.rgb(37, 79, 58))
    enableEdgeToEdge(statusBarStyle = bars, navigationBarStyle = bars)
    super.onCreate(savedInstanceState)
    window.decorView.setBackgroundColor(Color.rgb(245, 245, 239))
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    var originalMargins: ReceiptViewportInsets? = null

    // Wry owns the WebView and installs it directly in the activity content frame.
    // Apply insets to its existing layout parameters after attachment; never replace
    // the WebView or inject financial/UI data through a JavaScript bridge.
    ViewCompat.setOnApplyWindowInsetsListener(webView) { view, insets ->
      val params = view.layoutParams as? ViewGroup.MarginLayoutParams
        ?: return@setOnApplyWindowInsetsListener insets
      val original = originalMargins ?: ReceiptViewportInsets(
        params.leftMargin, params.topMargin, params.rightMargin, params.bottomMargin
      ).also { originalMargins = it }
      fun edges(type: Int): ReceiptViewportInsets {
        val value = insets.getInsets(type)
        return ReceiptViewportInsets(value.left, value.top, value.right, value.bottom)
      }
      val margins = receiptViewportMargins(
        original,
        edges(WindowInsetsCompat.Type.systemBars()),
        edges(WindowInsetsCompat.Type.displayCutout()),
        edges(WindowInsetsCompat.Type.ime())
      )
      if (params.leftMargin != margins.left || params.topMargin != margins.top ||
          params.rightMargin != margins.right || params.bottomMargin != margins.bottom) {
        params.setMargins(margins.left, margins.top, margins.right, margins.bottom)
        view.layoutParams = params
      }
      // The actual web viewport now excludes these areas. Consuming the applied
      // insets prevents a second offset through CSS env(safe-area-inset-*).
      WindowInsetsCompat.CONSUMED
    }
    webView.doOnAttach { ViewCompat.requestApplyInsets(it) }
  }
}
