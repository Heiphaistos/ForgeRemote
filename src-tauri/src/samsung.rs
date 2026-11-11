//! Samsung Tizen (2016+) : WebSocket wss://ip:8002. Le jeton reçu à l'appairage évite
//! la fenêtre de confirmation sur la TV aux envois suivants.
use crate::net::ws;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// Ouvre la session et attend l'accord de la TV. Retourne le socket et le jeton (nouveau ou renouvelé).
async fn connecter(ip: &str, token: Option<&str>) -> Result<(crate::net::Ws, Option<String>), String> {
    let name = STANDARD.encode("ForgeRemote");
    // Le jeton vient de la TV puis du disque : on n'accepte que des caractères sûrs pour une URL.
    let tok = token.filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c))).map(|t| format!("&token={t}")).unwrap_or_default();
    // 2016+ : wss 8002 (jeton) ; Tizen 2014-2015 : ws 8001 sans jeton (la TV redemande l'accord à chaque appairage).
    let mut s = match ws(&format!("wss://{ip}:8002/api/v2/channels/samsung.remote.control?name={name}{tok}"), 4).await {
        Ok(s) => s,
        Err(e) if e.contains("certificat de la TV a changé") => return Err(e),
        Err(_) => ws(&format!("ws://{ip}:8001/api/v2/channels/samsung.remote.control?name={name}"), 4).await.map_err(|_| "TV injoignable (ports 8002 et 8001 fermés : TV éteinte, ou modèle d'avant 2014 : choisis « Samsung ancien (2008-2013) »)".to_string())?,
    };
    // Sans jeton : l'utilisateur doit accepter sur la TV (30 s).
    let attente = Duration::from_secs(if token.is_some() { 5 } else { 30 });
    let nouveau = tokio::time::timeout(attente, async {
        while let Some(m) = s.next().await {
            let Ok(Message::Text(t)) = m else { continue };
            let v: Value = serde_json::from_str(&t).unwrap_or_default();
            match v["event"].as_str() {
                Some("ms.channel.connect") => return Ok(v["data"]["token"].as_str().map(String::from)),
                Some("ms.channel.timeout") | Some("ms.error") => return Err("refusé sur la TV".to_string()),
                _ => {}
            }
        }
        Err("TV déconnectée".to_string())
    })
    .await
    .map_err(|_| "pas de confirmation sur la TV".to_string())??;
    Ok((s, nouveau))
}

/// key=None : appairage seul. Retourne le jeton (nouveau ou renouvelé).
/// Sessions gardées ouvertes (ip -> socket, instant) : une touche ne refait plus TLS + authentification (latence ÷ 5 à 10).
static POOL: std::sync::Mutex<Option<std::collections::HashMap<String, (crate::net::Ws, std::time::Instant)>>> = std::sync::Mutex::new(None);
const VIE_MAX: Duration = Duration::from_secs(25);

fn rendre(ip: &str) -> Option<crate::net::Ws> {
    let (s, t) = POOL.lock().unwrap().as_mut()?.remove(ip)?;
    (t.elapsed() < VIE_MAX).then_some(s)
}
fn garder(ip: &str, s: crate::net::Ws) { POOL.lock().unwrap().get_or_insert_with(Default::default).insert(ip.to_string(), (s, std::time::Instant::now())); }

async fn clic(s: &mut crate::net::Ws, k: &str) -> Result<(), String> {
    let m = json!({"method":"ms.remote.control","params":{"Cmd":"Click","DataOfCmd":k,"Option":"false","TypeOfRemote":"SendRemoteKey"}});
    s.send(Message::text(m.to_string())).await.map_err(|e| e.to_string())
}

/// key=None : appairage seul. Retourne le jeton (nouveau ou renouvelé).
pub async fn send(ip: &str, token: Option<&str>, key: Option<&str>) -> Result<Option<String>, String> {
    let simple = key.filter(|k| !k.starts_with("TEXT:") && !k.starts_with("APP:"));
    if let Some(k) = simple {
        if let Some(mut s) = rendre(ip) { if clic(&mut s, k).await.is_ok() { garder(ip, s); return Ok(None); } }
    }
    if let Some(nom) = key.and_then(|k| k.strip_prefix("APP:")) {
        if lancer_rest(ip, nom).await? { return Ok(None); }
    }
    let (mut s, nouveau) = connecter(ip, token).await?;
    if let Some(txt) = key.and_then(|k| k.strip_prefix("TEXT:")) {
        // Saisie dans le champ de texte actif de la TV (texte en base64).
        let m = json!({"method":"ms.remote.control","params":{"Cmd":STANDARD.encode(txt),"DataOfCmd":"base64","TypeOfRemote":"SendInputString"}});
        s.send(Message::text(m.to_string())).await.map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    } else if let Some(nom) = key.and_then(|k| k.strip_prefix("APP:")) {
        lancer_app(&mut s, nom).await?;
    } else if let Some(k) = key {
        clic(&mut s, k).await?;
        garder(ip, s);
        return Ok(nouveau);
    }
    let _ = s.close(None).await;
    Ok(nouveau)
}

/// Applis courantes : (mot-clé, identifiant Samsung). Plusieurs identifiants possibles selon l'année de la TV (Netflix, Prime).
/// Vérifié sur une QN70F (Tizen 2025) : GET /api/v2/applications/<id> répond 200 + nom si installée, POST lance l'appli, sans jeton.
const CATALOGUE: [(&str, &str); 14] = [
    ("netflix", "3201907018807"), ("netflix", "11101200001"), ("youtube", "111299001912"), ("youtube tv", "3201707014489"), ("prime", "3201910019365"), ("prime", "3201512006785"),
    ("disney", "3201901017640"), ("plex", "3201512006963"), ("spotify", "3201606009684"), ("apple tv", "3201807016597"), ("rakuten", "3201511006428"),
    ("apple music", "3201908019041"), ("max", "3202301029760"), ("twitch", "3201803015944"),
];

fn http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_millis(2500)).build().map_err(|e| e.to_string())
}

/// Nom de l'appli si elle est installée (REST), sinon None.
async fn rest_nom(c: &reqwest::Client, ip: &str, id: &str) -> Option<String> {
    let r = c.get(format!("http://{ip}:8001/api/v2/applications/{id}")).send().await.ok()?;
    if !r.status().is_success() { return None; }
    let v: Value = r.json().await.ok()?;
    v["name"].as_str().map(|n| n.chars().take(40).collect())
}

/// Applis du catalogue installées sur la TV : [(id, nom)]. Marche aussi sur les TV récentes qui ne donnent plus leur liste.
pub async fn apps_rest(ip: &str) -> Vec<(String, String)> {
    let Ok(c) = http() else { return vec![] };
    let r = futures_util::future::join_all(CATALOGUE.iter().map(|(_, id)| { let c = c.clone(); async move { rest_nom(&c, ip, id).await.map(|n| (id.to_string(), n)) } })).await;
    r.into_iter().flatten().collect()
}

/// Lance une appli du catalogue par REST : Ok(true) = lancée, Ok(false) = pas dans le catalogue ou pas installée.
/// POST direct (404 = pas installée sous cet identifiant, on essaie le suivant) : un aller-retour de moins qu'un test préalable.
async fn lancer_rest(ip: &str, nom: &str) -> Result<bool, String> {
    let c = http()?;
    for (_, id) in CATALOGUE.iter().filter(|(cle, _)| nom.contains(cle) || cle.contains(nom)) {
        // Le démarrage d'une appli prend 3 à 5 s : au-delà de 1,5 s sans erreur, la TV a bien reçu l'ordre (vérifié : l'appli passe « running »).
        let r = match c.post(format!("http://{ip}:8001/api/v2/applications/{id}")).timeout(Duration::from_millis(1500)).send().await {
            Ok(r) => r,
            Err(e) if e.is_timeout() => return Ok(true),
            Err(e) => return Err(format!("TV injoignable : {e}")),
        };
        if r.status().is_success() { return Ok(true); }
        if r.status() != reqwest::StatusCode::NOT_FOUND { return Err(format!("la TV refuse de lancer l'application ({})", r.status())); }
    }
    Ok(false)
}

/// Identifiants Samsung (Tizen) des grandes applis, utilisés si la TV ne donne pas sa liste.
const ID_CONNUS: [(&str, &str); 5] = [("plex", "3201512006963"), ("netflix", "11101200001"), ("youtube", "111299001912"), ("prime", "3201512006785"), ("disney", "3201901017640")];

/// Lance l'application installée dont le nom contient `nom` (liste fournie par la TV : ed.installedApp.get).
async fn lancer_app(s: &mut crate::net::Ws, nom: &str) -> Result<(), String> {
    let q = json!({"method":"ms.channel.emit","params":{"event":"ed.installedApp.get","to":"host"}});
    s.send(Message::text(q.to_string())).await.map_err(|e| e.to_string())?;
    let apps = tokio::time::timeout(Duration::from_secs(5), async {
        while let Some(m) = s.next().await {
            let Ok(Message::Text(t)) = m else { continue };
            let v: Value = serde_json::from_str(&t).unwrap_or_default();
            if v["event"] == "ed.installedApp.get" { return v["data"]["data"].as_array().cloned(); }
        }
        None
    }).await.ok().flatten();
    let (id, kind) = match apps {
        Some(apps) => {
            let a = apps.iter().find(|a| a["name"].as_str().is_some_and(|n| n.to_lowercase().contains(nom)))
                .ok_or_else(|| format!("application « {nom} » non installée sur cette TV"))?;
            (a["appId"].as_str().ok_or("appId absent")?.to_string(), if a["app_type"] == 2 { "DEEP_LINK" } else { "NATIVE_LAUNCH" })
        }
        // Modèles récents : pas de liste d'applis -> identifiants Samsung connus des grandes applis.
        None => (ID_CONNUS.iter().find(|(n, _)| nom.contains(n)).map(|(_, id)| id.to_string())
            .ok_or("cette TV ne fournit pas la liste de ses applications (seules Plex, Netflix, YouTube, Prime et Disney+ sont connues)")?, "DEEP_LINK"),
    };
    let go = json!({"method":"ms.channel.emit","params":{"event":"ed.apps.launch","to":"host","data":{"appId":id,"action_type":kind}}});
    s.send(Message::text(go.to_string())).await.map_err(|e| e.to_string())?;
    tokio::time::sleep(Duration::from_millis(300)).await;
    Ok(())
}

/// Noms des applications installées : liste fournie par la TV (ed.installedApp.get, TV jusqu'à ~2021) complétée
/// par le catalogue testé en REST (TV récentes).
pub async fn apps(ip: &str, token: Option<&str>) -> Result<Vec<String>, String> {
    let mut noms: Vec<String> = apps_rest(ip).await.into_iter().map(|(_, n)| n).collect();
    if let Ok((mut s, _)) = connecter(ip, token).await {
        let q = json!({"method":"ms.channel.emit","params":{"event":"ed.installedApp.get","to":"host"}});
        if s.send(Message::text(q.to_string())).await.is_ok() {
            let liste = tokio::time::timeout(Duration::from_secs(4), async {
                while let Some(m) = s.next().await {
                    let Ok(Message::Text(t)) = m else { continue };
                    let v: Value = serde_json::from_str(&t).unwrap_or_default();
                    if v["event"] == "ed.installedApp.get" { return v["data"]["data"].as_array().cloned(); }
                }
                None
            }).await.ok().flatten().unwrap_or_default();
            noms.extend(liste.iter().filter_map(|a| a["name"].as_str()).map(|n| n.chars().take(40).collect::<String>()));
        }
        let _ = s.close(None).await;
    }
    if noms.is_empty() { return Err("cette TV ne fournit pas la liste de ses applications (les boutons Plex, Netflix, YouTube, Prime, Disney+, Spotify, Apple TV fonctionnent quand même)".into()); }
    Ok(noms)
}

#[cfg(test)]
mod tests {
    /// Mesure manuelle de latence contre une fausse TV sur 127.0.0.1:8002 (--ignored) : 1re touche (connexion) puis suivantes (session gardée).
    #[test]
    #[ignore]
    fn latence_touches() {
        crate::net::init(std::env::temp_dir().join("fr-lat"));
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        for i in 0..6 {
            let t = std::time::Instant::now();
            rt.block_on(super::send("127.0.0.1", Some("T"), Some("KEY_VOLUP"))).unwrap();
            println!("touche {i} : {} ms", t.elapsed().as_millis());
        }
    }

    /// Test manuel contre une vraie Samsung (IP_TV) : liste des applis puis lancement d'une appli (APPLI), sans jeton (REST).
    #[test]
    #[ignore]
    fn reel_applis() {
        crate::net::init(std::env::temp_dir().join("fr-reel"));
        let ip = std::env::var("IP_TV").unwrap();
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        println!("LISTE: {:?}", rt.block_on(super::apps_rest(&ip)));
        let t = std::time::Instant::now();
        println!("LANCEMENT {}: {:?} en {} ms", std::env::var("APPLI").unwrap(), rt.block_on(super::send(&ip, None, Some(&format!("APP:{}", std::env::var("APPLI").unwrap())))), t.elapsed().as_millis());
    }
}
