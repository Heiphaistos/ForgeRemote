package org.heiphaistos.forgeremote

import android.content.Context
import android.content.pm.PackageManager
import android.hardware.ConsumerIrManager
import android.net.wifi.WifiManager
import android.os.Build
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
  private var upd: UpdBridge? = null

  override fun onResume() {
    super.onResume()
    upd?.resume() // retour de l'écran « sources inconnues » : poursuit l'installation
  }

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
    // Android 17 (cible 37) bloque le réseau local sans cette autorisation runtime : sans elle,
    // ni l'IP seule ni le scan ne fonctionnent. Ignorée silencieusement sur les versions sans cette permission.
    val p = "android.permission.ACCESS_LOCAL_NETWORK"
    if (Build.VERSION.SDK_INT >= 36 && checkSelfPermission(p) != PackageManager.PERMISSION_GRANTED) {
      requestPermissions(arrayOf(p), 1)
    }
    // Le Wi-Fi Android jette le multicast (SSDP) sans verrou.
    (applicationContext.getSystemService(Context.WIFI_SERVICE) as? WifiManager)
      ?.createMulticastLock("forgeremote")?.apply { setReferenceCounted(false); acquire() }
  }

  override fun onWebViewCreate(webView: WebView) {
    val ir = getSystemService(Context.CONSUMER_IR_SERVICE) as? ConsumerIrManager
    webView.addJavascriptInterface(IrBridge(ir), "ForgeIR")
    webView.addJavascriptInterface(UpdBridge(this).also { upd = it }, "ForgeUpd")
    // Télécommande Bluetooth HID : Android 9+ seulement.
    if (Build.VERSION.SDK_INT >= 28) webView.addJavascriptInterface(BtBridge(this), "ForgeBT")
  }
}
