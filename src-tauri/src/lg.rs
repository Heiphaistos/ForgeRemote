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

/// Sessions gardées ouvertes (ip -> socket principal + socket pointeur, instant) : une touche ne refait plus l'enregistrement ni l'ouverture du pointeur.
static POOL: std::sync::Mutex<Option<std::collections::HashMap<String, (Ws, Ws, std::time::Instant)>>> = std::sync::Mutex::new(None);

fn rendre(ip: &str) -> Option<(Ws, Ws)> {
    let (s, b, t) = POOL.lock().unwrap().as_mut()?.remove(ip)?;
    (t.elapsed() < Duration::from_secs(25)).then_some((s, b))
}
fn garder(ip: &str, s: Ws, b: Ws) { POOL.lock().unwrap().get_or_insert_with(Default::default).insert(ip.to_string(), (s, b, std::time::Instant::now())); }

/// key=None : appairage seul. Retourne la clé client (nouvelle ou renouvelée).
pub async fn send(ip: &str, token: Option<&str>, key: Option<&str>) -> Result<Option<String>, String> {
    let bouton = key.filter(|k| !k.starts_with("TEXT:") && !k.starts_with("APP:") && !k.starts_with("SRC:") && *k != "POWER");
    if let Some(k) = bouton {
        if let Some((s, mut b)) = rendre(ip) {
            if b.send(Message::text(format!("type:button
name:{k}

"))).await.is_ok() { garder(ip, s, b); return Ok(None); }
        }
    }
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
        Some(k) if k.starts_with("TEXT:") => {
            // Clavier : insère le texte dans le champ actif (service IME de webOS).
            s.send(Message::text(json!({"type":"request","id":"txt","uri":"ssap://com.webos.service.ime/insertText","payload":{"text":&k[5..],"replace":0}}).to_string())).await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Some(k) if k.starts_with("SRC:") => {
            s.send(Message::text(json!({"type":"request","id":"src","uri":"ssap://tv/switchInput","payload":{"inputId":&k[4..]}}).to_string())).await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Some(k) if k.starts_with("APP:") => {
            // Application par son nom : liste des applis de la TV, puis lancement de celle qui correspond.
            let nom = &k[4..];
            s.send(Message::text(json!({"type":"request","id":"apps","uri":"ssap://com.webos.applicationManager/listLaunchPoints"}).to_string())).await.map_err(|e| e.to_string())?;
            let r = lire(&mut s, "apps", 5).await?;
            let id = r["payload"]["launchPoints"].as_array().and_then(|a| a.iter().find(|p| p["title"].as_str().is_some_and(|t| t.to_lowercase().contains(nom))))
                .and_then(|p| p["id"].as_str()).ok_or_else(|| format!("application « {nom} » non installée sur cette TV"))?.to_string();
            s.send(Message::text(json!({"type":"request","id":"go","uri":"ssap://system.launcher/launch","payload":{"id":id}}).to_string())).await.map_err(|e| e.to_string())?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        Some(k) => {
            s.send(Message::text(json!({"type":"request","id":"ptr","uri":"ssap://com.webos.service.networkinput/getPointerInputSocket"}).to_string())).await.map_err(|e| e.to_string())?;
            let r = lire(&mut s, "ptr", 5).await?;
            let path = r["payload"]["socketPath"].as_str().ok_or("socket pointeur absent")?;
            // L'URL vient de la TV : elle ne doit mener qu'à cette même TV (jamais une autre machine ni Internet).
            let hote = path.split("://").nth(1).unwrap_or("").split(['/', ':']).next();
            if !(path.starts_with("ws://") || path.starts_with("wss://")) || hote != Some(ip) { return Err("socket pointeur refusé (hôte inattendu)".into()); }
            let mut b = ws(path, 4).await?;
            b.send(Message::text(format!("type:button\nname:{k}\n\n"))).await.map_err(|e| e.to_string())?;
            garder(ip, s, b);
            return Ok(nouveau);
        }
    }
    let _ = s.close(None).await;
    Ok(nouveau)
}

/// Noms des applications de la TV (listLaunchPoints).
pub async fn apps(ip: &str, token: Option<&str>) -> Result<Vec<String>, String> {
    let mut s = match ws(&format!("wss://{ip}:3001"), 4).await { Ok(s) => s, Err(_) => ws(&format!("ws://{ip}:3000"), 4).await? };
    let mut p = json!({"forcePairing":false,"pairingType":"PROMPT","manifest":{"manifestVersion":1,"permissions":PERMS}});
    if let Some(t) = token { p["client-key"] = json!(t); }
    s.send(Message::text(json!({"type":"register","id":"reg","payload":p}).to_string())).await.map_err(|e| e.to_string())?;
    lire(&mut s, "reg", if token.is_some() { 5 } else { 30 }).await?;
    s.send(Message::text(json!({"type":"request","id":"apps","uri":"ssap://com.webos.applicationManager/listLaunchPoints"}).to_string())).await.map_err(|e| e.to_string())?;
    let r = lire(&mut s, "apps", 5).await?;
    let _ = s.close(None).await;
    Ok(r["payload"]["launchPoints"].as_array().map(|a| a.iter().filter_map(|p| p["title"].as_str()).map(|n| n.chars().take(40).collect()).collect()).unwrap_or_default())
}

/// Entrées de la TV (HDMI 1, AV...) : [(identifiant, nom)] via tv/getExternalInputList.
pub async fn sources(ip: &str, token: Option<&str>) -> Result<Vec<(String, String)>, String> {
    let mut s = match ws(&format!("wss://{ip}:3001"), 4).await { Ok(s) => s, Err(_) => ws(&format!("ws://{ip}:3000"), 4).await? };
    let mut p = json!({"forcePairing":false,"pairingType":"PROMPT","manifest":{"manifestVersion":1,"permissions":PERMS}});
    if let Some(t) = token { p["client-key"] = json!(t); }
    s.send(Message::text(json!({"type":"register","id":"reg","payload":p}).to_string())).await.map_err(|e| e.to_string())?;
    lire(&mut s, "reg", if token.is_some() { 5 } else { 30 }).await?;
    s.send(Message::text(json!({"type":"request","id":"src","uri":"ssap://tv/getExternalInputList"}).to_string())).await.map_err(|e| e.to_string())?;
    let r = lire(&mut s, "src", 5).await?;
    let _ = s.close(None).await;
    Ok(r["payload"]["devices"].as_array().map(|a| a.iter().filter_map(|d| Some((d["id"].as_str()?.to_string(), d["label"].as_str().unwrap_or(d["id"].as_str()?).chars().take(30).collect()))).collect()).unwrap_or_default())
}
