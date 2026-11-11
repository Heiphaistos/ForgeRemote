//! Samsung Tizen (2016+) : WebSocket wss://ip:8002. Le jeton reçu à l'appairage évite
//! la fenêtre de confirmation sur la TV aux envois suivants.
use crate::net::ws;
use base64::{engine::general_purpose::STANDARD, Engine};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

/// key=None : appairage seul. Retourne le jeton (nouveau ou renouvelé).
pub async fn send(ip: &str, token: Option<&str>, key: Option<&str>) -> Result<Option<String>, String> {
    let name = STANDARD.encode("ForgeRemote");
    // Le jeton vient de la TV puis du disque : on n'accepte que des caractères sûrs pour une URL.
    let tok = token.filter(|t| !t.is_empty() && t.chars().all(|c| c.is_ascii_alphanumeric() || "-._~".contains(c))).map(|t| format!("&token={t}")).unwrap_or_default();
    let mut s = ws(&format!("wss://{ip}:8002/api/v2/channels/samsung.remote.control?name={name}{tok}"), 4).await?;
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
    if let Some(txt) = key.and_then(|k| k.strip_prefix("TEXT:")) {
        // Saisie dans le champ de texte actif de la TV (texte en base64).
        let m = json!({"method":"ms.remote.control","params":{"Cmd":STANDARD.encode(txt),"DataOfCmd":"base64","TypeOfRemote":"SendInputString"}});
        s.send(Message::text(m.to_string())).await.map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(300)).await;
    } else if let Some(nom) = key.and_then(|k| k.strip_prefix("APP:")) {
        lancer_app(&mut s, nom).await?;
    } else if let Some(k) = key {
        let m = json!({"method":"ms.remote.control","params":{"Cmd":"Click","DataOfCmd":k,"Option":"false","TypeOfRemote":"SendRemoteKey"}});
        s.send(Message::text(m.to_string())).await.map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(120)).await; // laisse la TV lire avant fermeture
    }
    let _ = s.close(None).await;
    Ok(nouveau)
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
