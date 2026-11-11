package org.heiphaistos.forgeremote

import android.app.Activity
import android.app.PendingIntent
import android.content.BroadcastReceiver
import android.content.Context
import android.content.Intent
import android.content.IntentFilter
import android.content.pm.PackageInstaller
import android.os.Build
import androidx.core.content.ContextCompat
import android.net.Uri
import android.provider.Settings
import android.webkit.JavascriptInterface
import java.io.File
import java.net.HttpURLConnection
import java.net.URL
import java.security.MessageDigest

/**
 * Mise à jour Android depuis l'application : télécharge l'APK de la release, vérifie son SHA-256
 * (SHA256SUMS de la release) AVANT d'ouvrir l'installeur système. Android impose un écran de
 * confirmation « Installer » (impossible à supprimer sans droits système) ; aucun passage par GitHub.
 * Même clé de signature que l'APK installé : la mise à jour s'installe par-dessus, données conservées.
 */
class UpdBridge(private val a: Activity) {
  @Volatile private var st = ""

  companion object { const val ACTION = "org.heiphaistos.forgeremote.INSTALL_RESULT" }

  /** Résultat de l'installeur Android : demande de confirmation, succès, ou échec expliqué (jamais un silence). */
  private val resultat = object : BroadcastReceiver() {
    override fun onReceive(c: Context, i: Intent) {
      val code = i.getIntExtra(PackageInstaller.EXTRA_STATUS, -999)
      val msg = i.getStringExtra(PackageInstaller.EXTRA_STATUS_MESSAGE) ?: ""
      when (code) {
        PackageInstaller.STATUS_PENDING_USER_ACTION -> {
          @Suppress("DEPRECATION")
          val conf = if (Build.VERSION.SDK_INT >= 33) i.getParcelableExtra(Intent.EXTRA_INTENT, Intent::class.java) else i.getParcelableExtra<Intent>(Intent.EXTRA_INTENT)
          if (conf == null) st = "err:l'installeur Android n'a pas fourni son écran de confirmation" else { st = "ok"; a.startActivity(conf.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK)) }
        }
        PackageInstaller.STATUS_SUCCESS -> st = "fini"
        else -> st = "err:" + explique(code, msg)
      }
    }
  }

  init { ContextCompat.registerReceiver(a, resultat, IntentFilter(ACTION), ContextCompat.RECEIVER_NOT_EXPORTED) }

  private fun explique(code: Int, msg: String): String {
    val base = when (code) {
      PackageInstaller.STATUS_FAILURE_ABORTED -> "installation annulée (ou refusée dans l'avertissement Play Protect : choisis « Installer quand même »)"
      PackageInstaller.STATUS_FAILURE_BLOCKED -> "installation bloquée par Android ou Play Protect"
      PackageInstaller.STATUS_FAILURE_CONFLICT -> "conflit avec la version installée (signée avec une autre clé ?) : désinstalle ForgeRemote puis installe l'APK de la release"
      PackageInstaller.STATUS_FAILURE_INCOMPATIBLE -> "APK incompatible avec ce téléphone"
      PackageInstaller.STATUS_FAILURE_INVALID -> "APK invalide ou corrompu"
      PackageInstaller.STATUS_FAILURE_STORAGE -> "espace insuffisant sur le téléphone"
      else -> "échec de l'installation (code $code)"
    }
    return if (msg.isNotEmpty()) "$base [$msg]" else base
  }

  /** APK déjà téléchargé et vérifié, en attente de l'autorisation « sources inconnues ». */
  @Volatile private var pending: File? = null

  /** "" au repos, "dl" téléchargement, "perm" autorisation « sources inconnues » à accorder, "ok" installeur ouvert, "err:..." */
  @JavascriptInterface
  fun status(): String = st

  private fun get(u: String): HttpURLConnection =
    (URL(u).openConnection() as HttpURLConnection).apply { connectTimeout = 15000; readTimeout = 60000; setRequestProperty("User-Agent", "forgeremote-updater") }

  /** https, github.com, ce dépôt, chemin normalisé (« ../ » et « %2e » ne peuvent pas en sortir). */
  private fun sure(u: String): Boolean = try {
    val p = java.net.URI(u).normalize()
    p.scheme == "https" && p.host == "github.com" && p.port == -1 && p.userInfo == null &&
      (p.rawPath ?: "").startsWith("/Heiphaistos/ForgeRemote/releases/") && !(p.rawPath ?: "").lowercase().contains("%2e")
  } catch (e: Exception) { false }

  private fun hex(b: ByteArray): String = b.joinToString("") { Integer.toHexString((it.toInt() and 0xff) or 0x100).substring(1) }

  @JavascriptInterface
  fun install(url: String, name: String, wantSigned: String) {
    if (st == "dl") return
    st = "dl"
    Thread {
      try {
        require(sure(url) && Regex("[0-9a-f]{64}").matches(wantSigned) && Regex("[A-Za-z0-9._-]{1,100}").matches(name) && !name.contains("..")) { "URL non autorisée" }
        val want = wantSigned // somme issue de SHA256SUMS dont la signature Ed25519 a été vérifiée par le cœur Rust
        val dir = File(a.cacheDir, "update").apply { deleteRecursively(); mkdirs() }
        val f = File(dir, name)
        val md = MessageDigest.getInstance("SHA-256")
        get(url).inputStream.use { i -> f.outputStream().use { o ->
          val buf = ByteArray(65536); var n: Int; var total = 0L
          while (i.read(buf).also { n = it } > 0) { total += n; require(total < 150L * 1024 * 1024) { "fichier trop gros" }; md.update(buf, 0, n); o.write(buf, 0, n) }
        } }
        if (hex(md.digest()) != want) { f.delete(); error("intégrité refusée : SHA-256 différent, mise à jour annulée") }
        if (!a.packageManager.canRequestPackageInstalls()) {
          pending = f
          st = "perm"
          a.startActivity(Intent(Settings.ACTION_MANAGE_UNKNOWN_APP_SOURCES, Uri.parse("package:" + a.packageName)).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
          return@Thread
        }
        ouvrir(f)
      } catch (e: Exception) { st = "err:" + (e.message ?: e.javaClass.simpleName) }
    }.start()
  }

  /** À rappeler au retour dans l'app : si l'autorisation vient d'être accordée, ouvre l'installeur. */
  @JavascriptInterface
  fun resume() {
    val f = pending ?: return
    if (a.packageManager.canRequestPackageInstalls()) { pending = null; a.runOnUiThread { ouvrir(f) } }
  }

  /** Installation par session PackageInstaller : Android affiche sa confirmation, puis renvoie le résultat à `resultat`. */
  private fun ouvrir(f: File) {
    try {
      val pi = a.packageManager.packageInstaller
      val params = PackageInstaller.SessionParams(PackageInstaller.SessionParams.MODE_FULL_INSTALL).apply { setSize(f.length()) }
      val id = pi.createSession(params)
      pi.openSession(id).use { sess ->
        f.inputStream().use { i -> sess.openWrite("forgeremote.apk", 0, f.length()).use { o -> i.copyTo(o); sess.fsync(o) } }
        val pend = PendingIntent.getBroadcast(a, id, Intent(ACTION).setPackage(a.packageName), PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_MUTABLE)
        sess.commit(pend.intentSender)
      }
      st = "inst"
    } catch (e: Exception) { st = "err:" + (e.message ?: e.javaClass.simpleName) }
  }
}
