//! Mise à jour automatique : dernière release GitHub, intégrité SHA-256 (fichier SHA256SUMS de la
//! release) vérifiée AVANT toute installation. Installation silencieuse sous Windows (NSIS /S).
//! Ailleurs (Android, Linux) : l'utilisateur est renvoyé à la page de la release.
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

fn url_sure(u: &str) -> bool { u.starts_with(&format!("https://github.com/{REPO}/releases/download/")) }

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
    #[cfg(not(windows))]
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
    if !(url_sure(url) && url_sure(sums)) || name.contains(['/', '\\']) { return Err("URL de mise à jour non autorisée".into()); }
    let c = client()?;
    let sums_txt = c.get(sums).send().await.map_err(|e| e.to_string())?.text().await.map_err(|e| e.to_string())?;
    let want = attendu(&sums_txt, name).ok_or("somme SHA-256 absente de la release")?;
    let data = c.get(url).send().await.map_err(|e| e.to_string())?.bytes().await.map_err(|e| e.to_string())?;
    if data.len() > MAX { return Err("fichier trop gros".into()); }
    let got: String = Sha256::digest(&data).iter().map(|b| format!("{b:02x}")).collect();
    if got != want { return Err("intégrité refusée : SHA-256 différent, mise à jour annulée".into()); }
    let dir = std::env::temp_dir().join("forgeremote-update");
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
    #[cfg(not(windows))]
    { Err("installation automatique : Windows (et Android via l'application) seulement, ouvre la page de la release".into()) }
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
    fn urls() {
        assert!(url_sure("https://github.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
        assert!(!url_sure("https://evil.com/Heiphaistos/ForgeRemote/releases/download/v1/x"));
    }
}
