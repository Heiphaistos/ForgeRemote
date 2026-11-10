//! LG webOS : WebSocket SSAP (wss 3001, repli ws 3000), puis socket « pointeur » pour les touches.
use crate::net::{ws, Ws};
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::time::Duration;
use tokio_tungstenite::tungstenite::Message;

const PERMS: [&str; 6] = ["LAUNCH", "CONTROL_AUDIO", "CONTROL_POWER", "CONTROL_INPUT_TEXT", "CONTROL_MOUSE_AND_KEYBOARD", "CONTROL_INPUT_MEDIA_PLAYBACK"];

async fn lire(s: &mut Ws, id: &str, secs: u64) -> Result<Value, String> {
    tokio::time::timeout(Duration::from_secs(secs), async {
        while let Some(m) = s.next().await {
            let Ok(Message::Text(t)) = m else { continue };
            let v: Value = serde_json::from_str(&t).unwrap_or_default();
            if v["id"] == id && (v["type"] == "registered" || v["type"] == "response" && v["payload"]["pairingType"].is_null()) {
                return Ok(v);
            }
            if v["type"] == "error" { return Err(v["error"].as_str().unwrap_or("erreur LG").to_string()); }
        }
        Err("TV déconnectée".to_string())
    })
    .await
    .map_err(|_| "pas de confirmation sur la TV".to_string())?
}

/// key=None : appairage seul. Retourne la clé client (nouvelle ou renouvelée).
pub async fn send(ip: &str, token: Option<&str>, key: Option<&str>) -> Result<Option<String>, String> {
    let mut s = match ws(&format!("wss://{ip}:3001"), 4).await {
        Ok(s) => s,
        Err(_) => ws(&format!("ws://{ip}:3000"), 4).await?, // anciens webOS
    };
    let mut p = json!({"forcePairing":false,"pairingType":"PROMPT","manifest":{"manifestVersion":1,"permissions":PERMS}});
    if let Some(t) = token { p["client-key"] = json!(t); }
    s.send(Message::text(json!({"type":"register","id":"reg","payload":p}).to_string())).await.map_err(|e| e.to_string())?;
    let r = lire(&mut s, "reg", if token.is_some() { 5 } else { 30 }).await?;
    let nouveau = r["payload"]["client-key"].as_str().map(String::from);
    match key {
        None => {}
        Some("POWER") => {
            s.send(Message::text(json!({"type":"request","id":"off","uri":"ssap://system/turnOff"}).to_string())).await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
        Some(k) => {
            s.send(Message::text(json!({"type":"request","id":"ptr","uri":"ssap://com.webos.service.networkinput/getPointerInputSocket"}).to_string())).await.map_err(|e| e.to_string())?;
            let r = lire(&mut s, "ptr", 5).await?;
            let path = r["payload"]["socketPath"].as_str().ok_or("socket pointeur absent")?;
            let mut b = ws(path, 4).await?;
            b.send(Message::text(format!("type:button\nname:{k}\n\n"))).await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(120)).await;
            let _ = b.close(None).await;
        }
    }
    let _ = s.close(None).await;
    Ok(nouveau)
}
