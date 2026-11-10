//! Marques « HTTP simple » : Roku (ECP) et Philips (JointSpace v6).
use std::time::Duration;

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())
}

fn verdict(r: Result<reqwest::Response, reqwest::Error>) -> Result<(), String> {
    let r = r.map_err(|e| format!("TV injoignable : {e}"))?;
    if r.status().is_success() { Ok(()) } else { Err(format!("TV a répondu {} (modèle avec authentification ?)", r.status())) }
}

/// Roku : POST http://ip:8060/keypress/<Touche>
pub async fn roku(ip: &str, key: &str) -> Result<(), String> {
    verdict(client()?.post(format!("http://{ip}:8060/keypress/{key}")).send().await)
}

/// Philips : POST http://ip:1925/6/input/key {"key": ...}
pub async fn philips(ip: &str, key: &str) -> Result<(), String> {
    verdict(client()?.post(format!("http://{ip}:1925/6/input/key")).json(&serde_json::json!({"key": key})).send().await)
}
