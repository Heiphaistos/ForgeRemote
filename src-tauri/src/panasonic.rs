//! Panasonic Viera : SOAP/HTTP sur le port 55000 (modèles sans chiffrement, avant ~2018).
use std::time::Duration;

pub async fn send(ip: &str, key: &str) -> Result<(), String> {
    let body = format!(
        r#"<?xml version="1.0" encoding="utf-8"?><s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/" s:encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"><s:Body><u:X_SendKey xmlns:u="urn:panasonic-com:service:p00NetworkControl:1"><X_KeyEvent>{key}</X_KeyEvent></u:X_SendKey></s:Body></s:Envelope>"#
    );
    let r = reqwest::Client::builder().timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())?
        .post(format!("http://{ip}:55000/nrc/control_0"))
        .header("Content-Type", r#"text/xml; charset="utf-8""#)
        .header("SOAPACTION", r#""urn:panasonic-com:service:p00NetworkControl:1#X_SendKey""#)
        .body(body)
        .send().await.map_err(|e| format!("TV injoignable : {e}"))?;
    if r.status().is_success() { Ok(()) } else { Err(format!("TV a répondu {} (modèle récent chiffré ?)", r.status())) }
}
