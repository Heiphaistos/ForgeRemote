package org.heiphaistos.forgeremote

import android.annotation.SuppressLint
import android.app.Activity
import android.bluetooth.BluetoothAdapter
import android.bluetooth.BluetoothDevice
import android.bluetooth.BluetoothHidDevice
import android.bluetooth.BluetoothHidDeviceAppSdpSettings
import android.bluetooth.BluetoothManager
import android.bluetooth.BluetoothProfile
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import android.webkit.JavascriptInterface
import androidx.annotation.RequiresApi
import org.json.JSONArray
import org.json.JSONObject
import java.util.concurrent.Executors

/**
 * Le téléphone se déclare comme une télécommande Bluetooth HID (clavier + touches média) :
 * les Android TV, Fire TV et beaucoup d'autres TV l'acceptent sans protocole propriétaire.
 * Exposé à l'interface locale sous le nom `ForgeBT`. Android 9+ (API 28) requis.
 */
@SuppressLint("MissingPermission")
@RequiresApi(Build.VERSION_CODES.P)
class BtBridge(private val act: Activity) {
  private val adapter: BluetoothAdapter? =
    (act.getSystemService(Context.BLUETOOTH_SERVICE) as? BluetoothManager)?.adapter
  private var hid: BluetoothHidDevice? = null
  private var host: BluetoothDevice? = null
  private var registered = false
  private var starting = false
  private val pool = Executors.newSingleThreadExecutor()

  // Descripteur HID : rapport 1 = clavier (8 octets), rapport 2 = touches « consommateur » (16 bits).
  private val descriptor = intArrayOf(
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x85, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x06, 0x75, 0x08, 0x15, 0x00,
    0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
    0x05, 0x0C, 0x09, 0x01, 0xA1, 0x01, 0x85, 0x02, 0x15, 0x00, 0x26, 0xFF, 0x03, 0x19, 0x00, 0x2A, 0xFF, 0x03,
    0x75, 0x10, 0x95, 0x01, 0x81, 0x00, 0xC0,
  ).map { it.toByte() }.toByteArray()

  // touche -> (rapport, code d'utilisation HID). Clavier : page 0x07 ; consommateur : page 0x0C.
  private val keys: Map<String, Pair<Int, Int>> = buildMap {
    put("up", 1 to 0x52); put("down", 1 to 0x51); put("left", 1 to 0x50); put("right", 1 to 0x4F)
    put("ok", 1 to 0x28); put("exit", 1 to 0x29)
    "1234567890".forEachIndexed { i, c -> put(c.toString(), 1 to (0x1E + i)) }
    put("power", 2 to 0x30); put("home", 2 to 0x223); put("back", 2 to 0x224); put("menu", 2 to 0x40)
    put("vol_up", 2 to 0xE9); put("vol_down", 2 to 0xEA); put("mute", 2 to 0xE2)
    put("ch_up", 2 to 0x9C); put("ch_down", 2 to 0x9D); put("info", 2 to 0x60); put("guide", 2 to 0x8D)
    put("search", 2 to 0x221)
    // Couleurs, sous-titres, format d'image, 3D, dernière chaîne, TV, favoris, enregistrement : usages « consommateur » standard
    put("red", 2 to 0x69); put("green", 2 to 0x6A); put("blue", 2 to 0x6B); put("yellow", 2 to 0x6C)
    put("subtitle", 2 to 0x61); put("aspect", 2 to 0x6D); put("mode_3d", 2 to 0x6E); put("prev_ch", 2 to 0x83)
    put("tv", 2 to 0x89); put("fav", 2 to 0x22A); put("rec", 2 to 0xB2); put("apps", 2 to 0x223); put("play", 2 to 0xB0); put("pause", 2 to 0xB1); put("stop", 2 to 0xB7); put("rew", 2 to 0xB4); put("ff", 2 to 0xB3)
  }

  private val perms get() =
    if (Build.VERSION.SDK_INT >= 31) arrayOf("android.permission.BLUETOOTH_CONNECT", "android.permission.BLUETOOTH_ADVERTISE") else emptyArray()

  private fun permsOk() = perms.all { act.checkSelfPermission(it) == PackageManager.PERMISSION_GRANTED }

  /** Démarre le profil HID (une fois). false = pas possible (ancien Android, pas de Bluetooth, permission absente). */
  private fun ensure(): Boolean {
    if (Build.VERSION.SDK_INT < 28 || adapter == null || !adapter.isEnabled) return false
    if (!permsOk()) { act.requestPermissions(perms, 2); return false }
    if (hid != null || starting) return true
    starting = true
    adapter.getProfileProxy(act, object : BluetoothProfile.ServiceListener {
      override fun onServiceConnected(profile: Int, proxy: BluetoothProfile) {
        val h = proxy as BluetoothHidDevice
        hid = h
        val sdp = BluetoothHidDeviceAppSdpSettings("ForgeRemote", "Télécommande", "ForgeRemote", BluetoothHidDevice.SUBCLASS1_COMBO, descriptor)
        h.registerApp(sdp, null, null, pool, object : BluetoothHidDevice.Callback() {
          override fun onAppStatusChanged(pluggedDevice: BluetoothDevice?, registered: Boolean) { this@BtBridge.registered = registered }
          override fun onConnectionStateChanged(device: BluetoothDevice, state: Int) {
            if (state == BluetoothProfile.STATE_CONNECTED) host = device else if (host == device) host = null
          }
        })
      }
      override fun onServiceDisconnected(profile: Int) { hid = null; registered = false; host = null; starting = false }
    }, BluetoothProfile.HID_DEVICE)
    return true
  }

  /** Bluetooth utilisable sur ce téléphone (déclenche la demande d'autorisation si besoin). */
  @JavascriptInterface fun available(): Boolean = ensure()

  /** Appareils déjà appairés : [{name, address}]. */
  @JavascriptInterface fun devices(): String {
    val a = JSONArray()
    if (Build.VERSION.SDK_INT >= 28 && adapter != null && permsOk())
      adapter.bondedDevices.forEach { a.put(JSONObject().put("name", it.name ?: it.address).put("address", it.address)) }
    return a.toString()
  }

  /** Rend le téléphone visible 2 min : la TV peut alors l'ajouter comme accessoire Bluetooth. */
  @JavascriptInterface fun discoverable() {
    if (ensure()) act.startActivity(Intent(BluetoothAdapter.ACTION_REQUEST_DISCOVERABLE).putExtra(BluetoothAdapter.EXTRA_DISCOVERABLE_DURATION, 120))
  }

  /** "connected" | "ready" (enregistré, pas connecté) | "starting". */
  @JavascriptInterface fun status(): String = when {
    host != null -> "connected"
    registered -> "ready"
    else -> "starting"
  }

  @JavascriptInterface fun connect(address: String): Boolean {
    if (!ensure() || !registered) return false
    val dev = adapter?.getRemoteDevice(address) ?: return false
    return hid?.connect(dev) == true
  }

  /** Envoie une touche (appui puis relâchement). false = pas de TV connectée ou touche absente. */
  @JavascriptInterface fun send(key: String): Boolean {
    val h = hid ?: return false
    val d = host ?: return false
    val (rapport, usage) = keys[key] ?: return false
    if (rapport == 1) {
      h.sendReport(d, 1, ByteArray(8).also { it[2] = usage.toByte() })
      Thread.sleep(30)
      h.sendReport(d, 1, ByteArray(8))
    } else {
      h.sendReport(d, 2, byteArrayOf((usage and 0xFF).toByte(), (usage shr 8).toByte()))
      Thread.sleep(30)
      h.sendReport(d, 2, byteArrayOf(0, 0))
    }
    return true
  }

  /** Tape un texte dans le champ actif de la TV (clavier HID, touches US ; accents retirés). Asynchrone : true = envoi lancé. */
  @JavascriptInterface fun sendText(text: String): Boolean {
    val h = hid ?: return false
    val d = host ?: return false
    val plat = java.text.Normalizer.normalize(text, java.text.Normalizer.Form.NFD).replace(Regex("\\p{M}+"), "")
    pool.execute {
      for (c in plat.take(200)) {
        val (maj, usage) = hidChar(c) ?: continue
        h.sendReport(d, 1, ByteArray(8).also { it[0] = if (maj) 0x02 else 0; it[2] = usage.toByte() })
        Thread.sleep(25)
        h.sendReport(d, 1, ByteArray(8))
        Thread.sleep(25)
      }
    }
    return true
  }

  /** (majuscule ?, usage clavier page 0x07) d'un caractère ASCII usuel, ou null. */
  private fun hidChar(c: Char): Pair<Boolean, Int>? = when (c) {
    in 'a'..'z' -> false to (0x04 + (c - 'a'))
    in 'A'..'Z' -> true to (0x04 + (c - 'A'))
    in '1'..'9' -> false to (0x1E + (c - '1'))
    '0' -> false to 0x27
    ' ' -> false to 0x2C
    '\n' -> false to 0x28
    '.' -> false to 0x37
    ',' -> false to 0x36
    '-' -> false to 0x2D
    '_' -> true to 0x2D
    '/' -> false to 0x38
    '?' -> true to 0x38
    ';' -> false to 0x33
    ':' -> true to 0x33
    '\'' -> false to 0x34
    '@' -> true to 0x1F
    '!' -> true to 0x1E
    else -> null
  }
}
