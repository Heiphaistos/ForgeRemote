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
    let tok = token.map(|t| format!("&token={t}")).unwrap_or_default();
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
    if let Some(k) = key {
        let m = json!({"method":"ms.remote.control","params":{"Cmd":"Click","DataOfCmd":k,"Option":"false","TypeOfRemote":"SendRemoteKey"}});
        s.send(Message::text(m.to_string())).await.map_err(|e| e.to_string())?;
        tokio::time::sleep(Duration::from_millis(120)).await; // laisse la TV lire avant fermeture
    }
    let _ = s.close(None).await;
    Ok(nouveau)
}
