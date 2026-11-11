//! Mise à jour automatique : dernière release GitHub, intégrité SHA-256 (fichier SHA256SUMS de la
//! release) vérifiée AVANT toute installation. Installation silencieuse sous Windows (NSIS /S).
//! Portable Windows : exe remplacé ; AppImage Linux : fichier remplacé ; Android : pont Kotlin ; macOS, deb, rpm : page de la release.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

const REPO: &str = "Heiphaistos/ForgeRemote";
const MAX: usize = 150 * 1024 * 1024;

fn version(s: &str) -> Option<(u32, u32, u32)> {
    let mut p = s.trim_start_matches('v').split('.').map(|x| x.parse::<u32>().ok());
    Some((p.next()??, p.next()??, p.next()??))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(60)).user_agent("forgeremote-updater").build().map_err(|e| e.to_string())
}

fn attendu(sums: &str, nom: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        let (h, n) = l.trim().split_once(char::is_whitespace)?;
        (n.trim().trim_start_matches('*') == nom).then(|| h.to_lowercase())
    })
}

/// URL analysée (pas un simple préfixe : « ../ » ou « %2e » ne doivent pas sortir du dépôt) : https, github.com, ce dépôt.
pub fn url_sure(u: &str) -> bool {
    let Ok(p) = reqwest::Url::parse(u) else { return false };
    p.scheme() == "https" && p.host_str() == Some("github.com") && p.port().is_none() && p.username().is_empty() && p.password().is_none()
        && p.path().starts_with(&format!("/{REPO}/releases/")) && !p.path().to_lowercase().contains("%2e") && !p.path().split('/').any(|s| s == ".." || s == ".")
}

/// Nom de fichier de la release : lettres, chiffres, « . _ - » seulement (jamais de guillemet, &, %, espace...).
fn nom_sur(n: &str) -> bool {
    !n.is_empty() && n.len() <= 100 && !n.contains("..") && n.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c))
}

/// Some({version, page, kind, name, url, sums, apk}) si une version plus récente existe.
/// kind : "setup" (installeur NSIS), "portable" (exe seul, remplacé sur place), "android" (APK : installé par le
/// pont Kotlin), "manuel" (Linux : page de la release).
pub async fn check() -> Result<Option<Value>, String> {
    let r: Value = client()?.get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let tag = r["tag_name"].as_str().ok_or("aucune release")?;
    if version(tag) <= version(env!("CARGO_PKG_VERSION")) { return Ok(None); }
    let assets = r["assets"].as_array().cloned().unwrap_or_default();
    let trouve = |f: &dyn Fn(&str) -> bool| assets.iter()
        .find(|a| a["name"].as_str().is_some_and(f))
        .map(|a| (a["name"].clone(), a["browser_download_url"].clone()));
    let sums = trouve(&|n| n == "SHA256SUMS");
    let apk = trouve(&|n| n.ends_with("_android.apk"));
    #[cfg(windows)]
    let (kind, fichier) = if installeur_nsis() { ("setup", trouve(&|n| n.ends_with("-setup.exe"))) } else { ("portable", trouve(&|n| n.ends_with("_portable.exe"))) };
    #[cfg(target_os = "linux")]
    let (kind, fichier): (&str, Option<(Value, Value)>) = if std::env::var_os("APPIMAGE").is_some() { ("appimage", trouve(&|n| n.ends_with(".AppImage"))) } else { ("manuel", None) };
    #[cfg(not(any(windows, target_os = "linux")))]
    let (kind, fichier): (&str, Option<(Value, Value)>) = (if cfg!(target_os = "android") { "android" } else { "manuel" }, None);
    Ok(Some(json!({"version": tag.trim_start_matches('v'), "page": r["html_url"], "kind": kind,
        "name": fichier.as_ref().map(|e| &e.0), "url": fichier.as_ref().map(|e| &e.1),
        "apk": apk.as_ref().map(|e| json!({"name": e.0, "url": e.1})), "sums": sums.map(|s| s.1)})))
}

#[cfg(windows)]
pub fn installeur_nsis() -> bool { tauri::utils::platform::bundle_type() == Some(tauri::utils::config::BundleType::Nsis) }

/// Supprime l'ancien exe laissé par une mise à jour portable (au démarrage).
pub fn nettoyer() {
    if let Ok(exe) = std::env::current_exe() { let _ = std::fs::remove_file(exe.with_extension("old")); }
}

/// Télécharge, vérifie le SHA-256, puis installe : installeur silencieux (NSIS) ou remplacement de l'exe portable.
/// L'appelant quitte l'app ensuite ; la nouvelle version est relancée toute seule.
pub async fn install(url: &str, name: &str, sums: &str) -> Result<(), String> {
    if !(url_sure(url) && url_sure(sums) && nom_sur(name)) { return Err("URL de mise à jour non autorisée".into()); }
    let c = client()?;
    let sums_txt = c.get(sums).send().await.map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    let want = attendu(&sums_txt, name).ok_or("somme SHA-256 absente de la release")?;
    let data = c.get(url).send().await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    if data.len() > MAX { return Err("fichier trop gros".into()); }
    let got: String = Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect();
    if got != want { return Err("intégrité refusée : SHA-256 différent, mise à jour annulée".into()); }
    // Dossier à nom imprévisible : un autre processus ne peut pas viser le fichier entre vérification et lancement.
    let alea = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let dir = std::env::temp_dir().join(format!("forgeremote-update-{}-{alea}", std::process::id()));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let f = dir.join(name);
    std::fs::write(&f, &data).map_err(|e| e.to_string())?;
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let app = std::env::current_exe().map_err(|e| e.to_string())?;
        if installeur_nsis() {
            // Installeur silencieux (par utilisateur), puis relance : un cmd détaché attend la fin de
            // l'installation. Il survit à la fermeture de l'app, sans fenêtre console.
            let ligne = format!(r#"/D /S /C "start /wait "" "{}" /S && start "" "{}"""#, f.display(), app.display());
            std::process::Command::new("cmd").raw_arg(ligne).creation_flags(0x0800_0000).spawn().map_err(|e| e.to_string())?;
        } else {
            // Portable : un exe en cours d'exécution peut être renommé mais pas écrasé.
            let old = app.with_extension("old");
            let _ = std::fs::remove_file(&old);
            std::fs::rename(&app, &old).map_err(|e| format!("remplacement impossible (dossier protégé ?) : {e}"))?;
            if let Err(e) = std::fs::copy(&f, &app) { let _ = std::fs::rename(&old, &app); return Err(e.to_string()); }
            std::process::Command::new(&app).creation_flags(0x0800_0000 | 0x0000_0008).spawn().map_err(|e| e.to_string())?;
        }
        Ok(())
    }
    #[cfg(target_os = "linux")]
    {
        use std::os::unix::fs::PermissionsExt;
        let cible = std::env::var("APPIMAGE").map_err(|_| "installation automatique : AppImage seulement, ouvre la page de la release".to_string())?;
        let nouveau = format!("{cible}.new");
        std::fs::copy(&f, &nouveau).map_err(|e| format!("remplacement impossible (dossier protégé ?) : {e}"))?;
        std::fs::set_permissions(&nouveau, std::fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        std::fs::rename(&nouveau, &cible).map_err(|e| e.to_string())?; // remplace l'AppImage en cours d'exécution
        std::process::Command::new(&cible).spawn().map_err(|e| e.to_string())?;
        Ok(())
    }
    #[cfg(not(any(windows, target_os = "linux")))]
    { Err("installation automatique : Windows, AppImage Linux et Android seulement, ouvre la page de la release".into()) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn versions() {
        assert!(version("v0.10.0") > version("0.9.9"));
        assert!(version("0.1.0") <= version("0.1.0"));
        assert_eq!(version("abc"), None);
    }
    #[test]
    fn somme() {
        let s = "AAA  a.exe\r\nbbb *b-setup.exe\n";
        assert_eq!(attendu(s, "b-setup.exe").as_deref(), Some("bbb"));
        assert_eq!(attendu(s, "c"), None);
    }
    #[test]
    fn urls_et_noms_sans_contournement() {
        assert!(url_sure("https://github.com/Heiphaistos/ForgeRemote/releases/download/v1/x.exe"));
        assert!(!url_sure("https://github.com/Heiphaistos/ForgeRemote/releases/download/../../../evil/repo/releases/download/v1/x.exe"));
        assert!(!url_sure("https://github.com/Heiphaistos/ForgeRemote/releases/download/%2e%2e/%2e%2e/x"));
        assert!(!url_sure("https://github.com.evil.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
        assert!(!url_sure("https://user@github.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
        assert!(!url_sure("http://github.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
        assert!(nom_sur("ForgeRemote_0.7.0_x64-setup.exe") && !nom_sur("x\" & calc & \"") && !nom_sur("a/b") && !nom_sur("..") && !nom_sur(""));
    }
    #[test]
    fn urls() {
        assert!(url_sure("https://github.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
        assert!(!url_sure("https://evil.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
    }
}
