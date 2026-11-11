//! Marques expérimentales : Vizio SmartCast, Xiaomi Mi TV, LG Netcast (2012-2014) et Fire TV.
//! Protocoles d'après les bibliothèques publiques (pyvizio, pymitv, pylgnetcast, API locale Fire TV) :
//! vérifiés par des serveurs simulés (forme des requêtes), PAS sur de vraies TV.
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Mutex, time::Duration};

/// Jeton de première étape d'appairage Vizio, gardé en mémoire entre l'affichage du code et sa saisie : ip -> (port, jeton).
static VIZIO_REQ: Mutex<Option<HashMap<String, (u16, u64)>>> = Mutex::new(None);

fn client() -> Result<reqwest::Client, String> {
    // Les TV Vizio et Fire TV ont un certificat auto-signé ; l'IP est une adresse privée de la liste blanche.
    reqwest::Client::builder().danger_accept_invalid_certs(true).redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(5)).build().map_err(|e| e.to_string())
}

fn ok(r: Result<reqwest::Response, reqwest::Error>) -> Result<reqwest::Response, String> {
    let r = r.map_err(|e| format!("TV injoignable : {e}"))?;
    if r.status().is_success() { Ok(r) } else { Err(format!("TV : réponse {}", r.status())) }
}

fn entre<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    Some(&s[i..i + s[i..].find(b)?])
}

// ---------- Xiaomi Mi TV : GET http://ip:6095/controller?action=keyevent&keycode=<nom> ----------
pub fn xiaomi_url(ip: &str, port: u16, code: &str) -> String { format!("http://{ip}:{port}/controller?action=keyevent&keycode={code}") }

pub async fn xiaomi(ip: &str, code: &str) -> Result<(), String> { xiaomi_port(ip, 6095, code).await }
async fn xiaomi_port(ip: &str, port: u16, code: &str) -> Result<(), String> { ok(client()?.get(xiaomi_url(ip, port, code)).send().await).map(|_| ()) }

pub async fn xiaomi_vivante(ip: &str) -> bool {
    match client() { Ok(c) => c.get(format!("http://{ip}:6095/request?action=isalive")).send().await.is_ok_and(|r| r.status().is_success()), Err(_) => false }
}

// ---------- LG Netcast : POST /roap/api/auth puis /roap/api/command (XML), port 8080 ----------
const ENTETE: &str = r#"<?xml version="1.0" encoding="utf-8"?>"#;

pub fn netcast_auth(cle: Option<&str>) -> String {
    match cle { None => format!("{ENTETE}<auth><type>AuthKeyReq</type></auth>"), Some(k) => format!("{ENTETE}<auth><type>AuthReq</type><value>{k}</value></auth>") }
}
pub fn netcast_commande(session: &str, code: &str) -> String { format!("{ENTETE}<command><session>{session}</session><type>HandleKeyInput</type><value>{code}</value></command>") }

async fn netcast_post(c: &reqwest::Client, ip: &str, port: u16, chemin: &str, corps: String) -> Result<String, String> {
    ok(c.post(format!("http://{ip}:{port}/roap/api/{chemin}")).header("Content-Type", "application/atom+xml").body(corps).send().await)?.text().await.map_err(|e| e.to_string())
}

/// Affiche le code d'appairage (6 chiffres) sur la TV.
pub async fn netcast_afficher(ip: &str) -> Result<(), String> { netcast_post(&client()?, ip, 8080, "auth", netcast_auth(None)).await.map(|_| ()) }

pub async fn netcast(ip: &str, cle: Option<&str>, code: &str) -> Result<(), String> { netcast_port(ip, 8080, cle, code).await }
async fn netcast_port(ip: &str, port: u16, cle: Option<&str>, code: &str) -> Result<(), String> {
    let k = cle.filter(|k| !k.is_empty() && k.chars().all(|c| c.is_ascii_alphanumeric())).ok_or("TV non appairée : « Afficher le code sur la TV » puis saisis-le dans le profil")?;
    if !code.bytes().all(|b| b.is_ascii_digit()) { return Err("code de touche invalide".into()); }
    let c = client()?;
    let rep = netcast_post(&c, ip, port, "auth", netcast_auth(Some(k))).await?;
    let session = entre(&rep, "<session>", "</session>").filter(|s| s.chars().all(|c| c.is_ascii_alphanumeric())).ok_or("session LG Netcast refusée (code d'appairage faux ?)")?;
    netcast_post(&c, ip, port, "command", netcast_commande(session, code)).await.map(|_| ())
}

// ---------- Vizio SmartCast : HTTPS 7345 (ou 9000), PUT JSON, en-tête AUTH ----------
pub fn vizio_touche(code: &str) -> Result<Value, String> {
    let (set, c) = code.split_once(':').and_then(|(a, b)| Some((a.parse::<u32>().ok()?, b.parse::<u32>().ok()?))).ok_or("code Vizio invalide")?;
    Ok(json!({"KEYLIST":[{"CODESET":set,"CODE":c,"ACTION":"KEYPRESS"}]}))
}

const PORTS_VIZIO: [u16; 2] = [7345, 9000];

async fn vizio_put(c: &reqwest::Client, ip: &str, port: u16, chemin: &str, auth: Option<&str>, corps: &Value) -> Result<Value, String> {
    let mut r = c.put(format!("https://{ip}:{port}{chemin}")).json(corps);
    if let Some(a) = auth { r = r.header("AUTH", a); }
    ok(r.send().await)?.json().await.map_err(|e| e.to_string())
}

/// Affiche un code à 4 chiffres sur la TV et mémorise le jeton de la première étape.
pub async fn vizio_afficher(ip: &str) -> Result<(), String> {
    let c = client()?;
    let corps = json!({"DEVICE_NAME":"ForgeRemote","DEVICE_ID":"forgeremote"});
    let mut derniere = String::new();
    for p in PORTS_VIZIO {
        match vizio_put(&c, ip, p, "/pairing/start", None, &corps).await {
            Ok(v) => {
                let t = v["ITEM"]["PAIRING_REQ_TOKEN"].as_u64().ok_or("réponse Vizio inattendue")?;
                VIZIO_REQ.lock().unwrap().get_or_insert_with(HashMap::new).insert(ip.to_string(), (p, t));
                return Ok(());
            }
            Err(e) => derniere = e,
        }
    }
    Err(derniere)
}

/// Termine l'appairage avec le code affiché : retourne le jeton AUTH à conserver.
pub async fn vizio_finir(ip: &str, pin: &str) -> Result<String, String> {
    if pin.is_empty() || pin.len() > 8 || !pin.bytes().all(|b| b.is_ascii_digit()) { return Err("code invalide (chiffres)".into()); }
    let (port, t) = VIZIO_REQ.lock().unwrap().as_ref().and_then(|m| m.get(ip).copied()).ok_or("clique d'abord sur « Afficher le code sur la TV »")?;
    let corps = json!({"DEVICE_ID":"forgeremote","CHALLENGE_TYPE":1,"RESPONSE_VALUE":pin,"PAIRING_REQ_TOKEN":t});
    let v = vizio_put(&client()?, ip, port, "/pairing/pair", None, &corps).await?;
    v["ITEM"]["AUTH_TOKEN"].as_str().filter(|a| a.len() <= 64 && a.chars().all(|c| c.is_ascii_alphanumeric())).map(String::from).ok_or_else(|| "code refusé par la TV".to_string())
}

pub async fn vizio(ip: &str, auth: Option<&str>, code: &str) -> Result<(), String> {
    let a = auth.filter(|a| !a.is_empty()).ok_or("TV non appairée : « Afficher le code sur la TV » puis saisis-le dans le profil")?;
    let corps = vizio_touche(code)?;
    let c = client()?;
    let mut e = String::new();
    for p in PORTS_VIZIO {
        match vizio_put(&c, ip, p, "/key_command/", Some(a), &corps).await { Ok(_) => return Ok(()), Err(x) => e = x }
    }
    Err(e)
}

pub async fn vizio_vivante(ip: &str) -> bool {
    let Ok(c) = client() else { return false };
    for p in PORTS_VIZIO {
        if let Ok(r) = c.get(format!("https://{ip}:{p}/state/device/deviceinfo")).send().await { if r.text().await.is_ok_and(|t| t.contains("STATUS")) { return true; } }
    }
    false
}

// ---------- Fire TV : API locale HTTPS 8080 (appairage par code, en-têtes X-Api-Key / X-Client-Token) ----------
const CLE_API_FIRE: &str = "0987654321";

/// (chemin, corps JSON) d'une commande Fire TV — noms d'actions et corps relevés dans la table de l'appli Roku
/// (dpad_*, select, home, back, menu, sleep, volume_up/down, mute ; media : play, pause, scan {direction} ; chiffres : texte).
pub fn firetv_requete(code: &str) -> Result<(String, Option<String>), String> {
    let nom = |n: &str| !n.is_empty() && n.len() <= 24 && n.chars().all(|c| c.is_ascii_lowercase() || c == '_');
    if let Some(d) = code.strip_prefix("scan:") {
        return if d == "forward" || d == "back" { Ok(("/v1/media?action=scan".into(), Some(format!(r#"{{"direction":"{d}","keyAction":{{"keyActionType":"keyDownUp"}}}}"#)))) } else { Err("direction Fire TV invalide".into()) };
    }
    if let Some(t) = code.strip_prefix("text:") {
        return if t.len() == 1 && t.chars().all(|c| c.is_ascii_digit()) { Ok(("/v1/FireTV/text".into(), Some(format!(r#"{{"text":"{t}"}}"#)))) } else { Err("texte Fire TV invalide".into()) };
    }
    match code.strip_prefix("media:") {
        Some(a) if nom(a) => Ok((format!("/v1/media?action={a}"), None)),
        None if nom(code) => Ok((format!("/v1/FireTV?action={code}"), None)),
        _ => Err("action Fire TV invalide".into()),
    }
}

pub async fn firetv_afficher(ip: &str) -> Result<(), String> {
    ok(client()?.post(format!("https://{ip}:8080/v1/FireTV/pin/display")).header("X-Api-Key", CLE_API_FIRE).json(&json!({"friendlyName":"ForgeRemote"})).send().await).map(|_| ())
}

pub async fn firetv_finir(ip: &str, pin: &str) -> Result<String, String> {
    if pin.is_empty() || pin.len() > 8 || !pin.bytes().all(|b| b.is_ascii_digit()) { return Err("code invalide (chiffres)".into()); }
    let v: Value = ok(client()?.post(format!("https://{ip}:8080/v1/FireTV/pin/verify")).header("X-Api-Key", CLE_API_FIRE).json(&json!({"pin":pin})).send().await)?.json().await.map_err(|e| e.to_string())?;
    v["description"].as_str().filter(|t| t.len() <= 128 && t.chars().all(|c| c.is_ascii_alphanumeric() || "-_=.".contains(c))).map(String::from).ok_or_else(|| "code refusé par la TV".to_string())
}

pub async fn firetv(ip: &str, jeton: Option<&str>, code: &str) -> Result<(), String> {
    let t = jeton.filter(|t| !t.is_empty()).ok_or("TV non appairée : « Afficher le code sur la TV » puis saisis-le dans le profil")?;
    let (chemin, corps) = firetv_requete(code)?;
    let mut r = client()?.post(format!("https://{ip}:8080{chemin}")).header("X-Api-Key", CLE_API_FIRE).header("X-Client-Token", t);
    if let Some(c) = corps { r = r.header("Content-Type", "application/json").body(c); }
    ok(r.send().await).map(|_| ())
}

// ---------- Appairage par code (Vizio, LG Netcast, Fire TV) ----------
pub async fn code_afficher(brand: &str, ip: &str) -> Result<(), String> {
    match brand { "vizio" => vizio_afficher(ip).await, "lgnetcast" => netcast_afficher(ip).await, "firetv" => firetv_afficher(ip).await, "androidtv" => crate::androidtv::appairage_debut(ip).await, _ => Err("cette marque ne demande pas de code".into()) }
}

/// Jeton à enregistrer dans le profil (Vizio, Fire TV) ; pour LG Netcast, le code lui-même sert de clé.
pub async fn code_finir(brand: &str, ip: &str, pin: &str) -> Result<String, String> {
    match brand {
        "vizio" => vizio_finir(ip, pin).await,
        "firetv" => firetv_finir(ip, pin).await,
        "androidtv" => crate::androidtv::appairage_fin(ip, pin).await.map(|_| "appaire".to_string()),
        "lgnetcast" if !pin.is_empty() && pin.len() <= 12 && pin.chars().all(|c| c.is_ascii_alphanumeric()) => Ok(pin.to_string()),
        _ => Err("code invalide".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    /// Faux serveur HTTP : répond `corps` et enregistre chaque requête (ligne + en-têtes + corps).
    fn faux(corps: &'static str) -> (u16, std::sync::Arc<Mutex<Vec<String>>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let vu = std::sync::Arc::new(Mutex::new(vec![]));
        let v2 = vu.clone();
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut b = [0u8; 4096];
                let n = s.read(&mut b).unwrap_or(0);
                v2.lock().unwrap().push(String::from_utf8_lossy(&b[..n]).to_string());
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{corps}", corps.len());
            }
        });
        (port, vu)
    }
    fn rt() -> tokio::runtime::Runtime { tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap() }

    #[test]
    fn xiaomi_requete() {
        let (p, vu) = faux("{}");
        rt().block_on(xiaomi_port("127.0.0.1", p, "volumeup")).unwrap();
        assert!(vu.lock().unwrap()[0].starts_with("GET /controller?action=keyevent&keycode=volumeup HTTP/1.1"));
    }

    #[test]
    fn netcast_deux_requetes_session_puis_commande() {
        let (p, vu) = faux("<?xml version=\"1.0\"?><envelope><session>abc123</session></envelope>");
        rt().block_on(netcast_port("127.0.0.1", p, Some("123456"), "24")).unwrap();
        let v = vu.lock().unwrap();
        assert!(v[0].starts_with("POST /roap/api/auth ") && v[0].contains("<type>AuthReq</type><value>123456</value>"));
        assert!(v[1].starts_with("POST /roap/api/command ") && v[1].contains("<session>abc123</session><type>HandleKeyInput</type><value>24</value>"));
        assert!(rt().block_on(netcast_port("127.0.0.1", p, None, "24")).is_err(), "sans appairage");
        assert!(rt().block_on(netcast_port("127.0.0.1", p, Some("12 34"), "24")).is_err(), "clé hostile");
        assert!(rt().block_on(netcast_port("127.0.0.1", p, Some("1234"), "24<x>")).is_err(), "code hostile");
    }

    #[test]
    fn formats_vizio_et_firetv() {
        assert_eq!(vizio_touche("5:1").unwrap(), json!({"KEYLIST":[{"CODESET":5,"CODE":1,"ACTION":"KEYPRESS"}]}));
        assert!(vizio_touche("x").is_err() && vizio_touche("5:").is_err());
        assert_eq!(firetv_requete("dpad_up").unwrap(), ("/v1/FireTV?action=dpad_up".to_string(), None));
        assert_eq!(firetv_requete("media:play").unwrap().0, "/v1/media?action=play");
        let (c, b) = firetv_requete("scan:forward").unwrap();
        assert_eq!((c.as_str(), b.as_deref()), ("/v1/media?action=scan", Some(r#"{"direction":"forward","keyAction":{"keyActionType":"keyDownUp"}}"#)));
        assert_eq!(firetv_requete("text:7").unwrap(), ("/v1/FireTV/text".to_string(), Some(r#"{"text":"7"}"#.to_string())));
        assert!(firetv_requete("../x").is_err() && firetv_requete("a&b=1").is_err() && firetv_requete("media:").is_err() && firetv_requete("text:\"}").is_err());
    }
}
