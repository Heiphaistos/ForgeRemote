package org.heiphaistos.forgeremote

import android.content.Context
import android.hardware.ConsumerIrManager
import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import org.json.JSONArray

/** Pont JS -> émetteur infrarouge du téléphone (si présent). Exposé à l'interface locale seulement. */
class IrBridge(private val ir: ConsumerIrManager?) {
  @JavascriptInterface
  fun hasIr(): Boolean = ir?.hasIrEmitter() == true

  @JavascriptInterface
  fun transmit(freq: Int, patternJson: String) {
    val a = JSONArray(patternJson)
    ir?.transmit(freq, IntArray(a.length()) { a.getInt(it) })
  }
}

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    val ir = getSystemService(Context.CONSUMER_IR_SERVICE) as? ConsumerIrManager
    webView.addJavascriptInterface(IrBridge(ir), "ForgeIR")
  }
}
