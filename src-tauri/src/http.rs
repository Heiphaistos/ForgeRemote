//! Marques « HTTP simple » : Roku (ECP) et Philips (JointSpace v6).
use std::time::Duration;

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())
}

fn verdict(r: Result<reqwest::Response, reqwest::Error>) -> Result<(), String> {
    let r = r.map_err(|e| format!("TV injoignable : {e}"))?;
    if r.status().is_success() { Ok(()) } else { Err(format!("TV a répondu {} (modèle avec authentification ?)", r.status())) }
}

/// Pourcentage-encode un caractère pour l'URL « Lit_<car> » de Roku.
fn pct(c: char) -> String { c.to_string().bytes().map(|b| if b.is_ascii_alphanumeric() { (b as char).to_string() } else { format!("%{b:02X}") }).collect() }

/// Roku : tape le texte dans le champ actif, un caractère à la fois (POST /keypress/Lit_<car>).
pub async fn roku_text(ip: &str, text: &str) -> Result<(), String> {
    let c = client()?;
    for ch in text.chars() { verdict(c.post(format!("http://{ip}:8060/keypress/Lit_{}", pct(ch))).send().await)?; }
    Ok(())
}

/// Identifiant de la chaîne dont le nom contient `nom` (réponse XML de GET /query/apps).
fn roku_app(xml: &str, nom: &str) -> Option<String> {
    xml.split("<app ").skip(1).find_map(|c| {
        let id = c.split("id=\"").nth(1)?.split('"').next()?;
        let titre = c.split_once('>')?.1.split("</app>").next()?;
        (id.bytes().all(|b| b.is_ascii_digit()) && titre.to_lowercase().contains(nom)).then(|| id.to_string())
    })
}

/// Roku : POST http://ip:8060/keypress/<Touche>, ou « APP:nom » = lance la chaîne installée de ce nom.
pub async fn roku(ip: &str, key: &str) -> Result<(), String> {
    if let Some(nom) = key.strip_prefix("APP:") {
        let c = client()?;
        let xml = c.get(format!("http://{ip}:8060/query/apps")).send().await.map_err(|e| format!("TV injoignable : {e}"))?
            .text().await.map_err(|e| e.to_string())?;
        let id = roku_app(&xml, nom).ok_or_else(|| format!("application « {nom} » non installée sur cette TV"))?;
        return verdict(c.post(format!("http://{ip}:8060/launch/{id}")).send().await);
    }
    verdict(client()?.post(format!("http://{ip}:8060/keypress/{key}")).send().await)
}

/// Philips : POST http://ip:1925/6/input/key {"key": ...}
pub async fn philips(ip: &str, key: &str) -> Result<(), String> {
    verdict(client()?.post(format!("http://{ip}:1925/6/input/key")).json(&serde_json::json!({"key": key})).send().await)
}

#[cfg(test)]
mod tests {
    #[test]
    fn pct_encode() {
        assert_eq!((super::pct('a'), super::pct(' '), super::pct('é')), ("a".into(), "%20".into(), "%C3%A9".into()));
    }

    #[test]
    fn roku_trouve_par_nom() {
        let x = r#"<apps><app id="12" type="appl" version="4.1">Netflix</app><app id="13535" type="appl" version="2">Plex - Stream for Free</app></apps>"#;
        assert_eq!(super::roku_app(x, "plex").as_deref(), Some("13535"));
        assert_eq!(super::roku_app(x, "netflix").as_deref(), Some("12"));
        assert_eq!(super::roku_app(x, "disney"), None);
    }
}
