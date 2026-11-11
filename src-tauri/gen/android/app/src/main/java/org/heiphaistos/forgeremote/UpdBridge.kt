package org.heiphaistos.forgeremote

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.provider.Settings
import android.webkit.JavascriptInterface
import androidx.core.content.FileProvider
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

  /** APK déjà téléchargé et vérifié, en attente de l'autorisation « sources inconnues ». */
  @Volatile private var pending: File? = null

  /** "" au repos, "dl" téléchargement, "perm" autorisation « sources inconnues » à accorder, "ok" installeur ouvert, "err:..." */
  @JavascriptInterface
  fun status(): String = st

  private fun get(u: String): HttpURLConnection =
    (URL(u).openConnection() as HttpURLConnection).apply { connectTimeout = 15000; readTimeout = 60000; setRequestProperty("User-Agent", "forgeremote-updater") }

  private fun hex(b: ByteArray): String = b.joinToString("") { Integer.toHexString((it.toInt() and 0xff) or 0x100).substring(1) }

  @JavascriptInterface
  fun install(url: String, name: String, sums: String) {
    if (st == "dl") return
    st = "dl"
    Thread {
      try {
        val base = "https://github.com/Heiphaistos/ForgeRemote/releases/download/"
        require(url.startsWith(base) && sums.startsWith(base) && !name.contains('/') && !name.contains('\\')) { "URL non autorisée" }
        val txt = get(sums).inputStream.bufferedReader().readText()
        val want = txt.lines().firstNotNullOfOrNull { l ->
          val p = l.trim().split(Regex("\\s+"), 2)
          if (p.size == 2 && p[1].trim().trimStart('*') == name) p[0].lowercase() else null
        } ?: error("somme SHA-256 absente de la release")
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

  private fun ouvrir(f: File) {
    val uri = FileProvider.getUriForFile(a, a.packageName + ".fileprovider", f)
    a.startActivity(Intent(Intent.ACTION_VIEW).setDataAndType(uri, "application/vnd.android.package-archive")
      .addFlags(Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_ACTIVITY_NEW_TASK))
    st = "ok"
  }
}
