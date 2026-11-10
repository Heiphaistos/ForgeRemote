//! Mise à jour automatique : dernière release GitHub, intégrité SHA-256 (fichier SHA256SUMS de la
//! release) vérifiée AVANT toute installation. Installation silencieuse sous Windows (NSIS /S).
//! Ailleurs (Android, Linux) : l'utilisateur est renvoyé à la page de la release.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

const REPO: &str = "Heiphaistos44/ForgeRemote";
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

/// Some({version, page, name, url, sums}) si une version plus récente existe.
pub async fn check() -> Result<Option<Value>, String> {
    let r: Value = client()?.get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .send().await.map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let tag = r["tag_name"].as_str().ok_or("aucune release")?;
    if version(tag) <= version(env!("CARGO_PKG_VERSION")) { return Ok(None); }
    let assets = r["assets"].as_array().cloned().unwrap_or_default();
    let trouve = |f: &dyn Fn(&str) -> bool| assets.iter()
        .find(|a| a["name"].as_str().is_some_and(f))
        .map(|a| (a["name"].clone(), a["browser_download_url"].clone()));
    let exe = trouve(&|n| n.ends_with("-setup.exe"));
    let sums = trouve(&|n| n == "SHA256SUMS");
    Ok(Some(json!({"version": tag.trim_start_matches('v'), "page": r["html_url"],
        "name": exe.as_ref().map(|e| &e.0), "url": exe.as_ref().map(|e| &e.1), "sums": sums.map(|s| s.1)})))
}

/// Télécharge, vérifie le SHA-256, lance l'installeur silencieux. L'appelant quitte l'app ensuite.
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
    { std::process::Command::new(&f).arg("/S").spawn().map_err(|e| e.to_string())?; Ok(()) }
    #[cfg(not(windows))]
    { Err("installation automatique : Windows seulement, ouvre la page de la release".into()) }
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
        assert!(url_sure("https://github.com/Heiphaistos44/ForgeRemote/releases/download/v1/x"));
        assert!(!url_sure("https://evil.com/Heiphaistos44/ForgeRemote/releases/download/v1/x"));
    }
}
