//! Sony Bravia IRCC-IP. La clé PSK (réglée sur la TV : Réseau > Contrôle IP > Pre-Shared Key)
//! est stockée dans `token`. Les codes viennent de la TV elle-même (getRemoteControllerInfo) :
//! aucun code en dur, donc valable pour tous les modèles.
use serde_json::{json, Value};
use std::time::Duration;

pub async fn send(ip: &str, psk: Option<&str>, key: &str) -> Result<(), String> {
    let psk = psk.ok_or("clé PSK Sony manquante (réglages réseau de la TV)")?;
    let c = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())?;
    let info: Value = c.post(format!("http://{ip}/sony/system")).header("X-Auth-PSK", psk)
        .json(&json!({"method":"getRemoteControllerInfo","id":1,"params":[],"version":"1.0"}))
        .send().await.map_err(|e| format!("TV injoignable : {e}"))?
        .json().await.map_err(|_| "réponse Sony illisible (PSK refusée ?)".to_string())?;
    let code = info["result"][1].as_array().into_iter().flatten()
        .find(|e| e["name"] == key).and_then(|e| e["value"].as_str())
        .ok_or_else(|| format!("code « {key} » absent de cette TV"))?;
    let body = format!(r#"<?xml version="1.0"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body><u:X_SendIRCC xmlns:u="urn:schemas-sony-com:service:IRCC:1"><IRCCCode>{}</IRCCCode></u:X_SendIRCC></s:Body></s:Envelope>"#, code.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;"));
    let r = c.post(format!("http://{ip}/sony/ircc")).header("X-Auth-PSK", psk)
        .header("Content-Type", "text/xml; charset=UTF-8")
        .header("SOAPACTION", r#""urn:schemas-sony-com:service:IRCC:1#X_SendIRCC""#)
        .body(body).send().await.map_err(|e| e.to_string())?;
    if r.status().is_success() { Ok(()) } else { Err(format!("TV a répondu {}", r.status())) }
}
