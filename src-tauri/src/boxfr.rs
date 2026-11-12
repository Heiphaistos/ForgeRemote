//! Box TV françaises : Freebox Player (API HTTP « télécommande réseau », code à 8 chiffres affiché dans les réglages
//! de la box) et décodeur Orange TV (HTTP port 8080, codes de touches numériques).
//! Sources : python-freebox-api (remote.py), python-liveboxplaytv (keys.py). Vérifié par serveurs simulés, PAS sur une vraie box.
//! Bouygues (Bbox Miami) et SFR récentes sont sous Android TV : utiliser la marque « Android TV ».
use std::time::Duration;

/// id de touche -> nom Freebox.
const FREEBOX: &[(&str, &str)] = &[
    ("power", "power"), ("0", "0"), ("1", "1"), ("2", "2"), ("3", "3"), ("4", "4"), ("5", "5"), ("6", "6"), ("7", "7"), ("8", "8"), ("9", "9"),
    ("up", "up"), ("down", "down"), ("left", "left"), ("right", "right"), ("ok", "ok"), ("back", "back"), ("home", "home"), ("menu", "options"),
    ("vol_up", "vol_inc"), ("vol_down", "vol_dec"), ("mute", "mute"), ("ch_up", "prgm_inc"), ("ch_down", "prgm_dec"), ("prev_ch", "swap"),
    ("info", "info"), ("guide", "epg"), ("ch_list", "list"), ("tv", "tv"), ("apps", "media"), ("play", "play"), ("pause", "play"),
    ("rew", "bwd"), ("ff", "fwd"), ("rec", "rec"), ("red", "red"), ("green", "green"), ("yellow", "yellow"), ("blue", "blue"),
];

/// id de touche -> code numérique Orange.
const ORANGE: &[(&str, &str)] = &[
    ("power", "116"), ("0", "512"), ("1", "513"), ("2", "514"), ("3", "515"), ("4", "516"), ("5", "517"), ("6", "518"), ("7", "519"), ("8", "520"), ("9", "521"),
    ("up", "103"), ("down", "108"), ("left", "105"), ("right", "106"), ("ok", "352"), ("back", "158"), ("menu", "139"),
    ("vol_up", "115"), ("vol_down", "114"), ("mute", "113"), ("ch_up", "402"), ("ch_down", "403"),
    ("play", "164"), ("pause", "164"), ("rew", "168"), ("ff", "159"), ("rec", "167"), ("apps", "393"),
];

fn table(brand: usize) -> &'static [(&'static str, &'static str)] { if brand == 12 { FREEBOX } else { ORANGE } }

/// Touches gérées (brand = 12 Freebox, 13 Orange).
pub fn supported(brand: usize) -> Vec<&'static str> { table(brand).iter().map(|k| k.0).collect() }

pub fn code(id: &str, brand: usize) -> Result<String, String> {
    table(brand).iter().find(|k| k.0 == id).map(|k| k.1.to_string()).ok_or_else(|| format!("touche « {id} » non gérée par cette box"))
}

fn client() -> Result<reqwest::Client, String> {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_secs(4)).build().map_err(|e| e.to_string())
}

fn ok(r: Result<reqwest::Response, reqwest::Error>) -> Result<(), String> {
    let r = r.map_err(|e| format!("box injoignable : {e}"))?;
    if r.status().is_success() { Ok(()) } else { Err(format!("box : réponse {}", r.status())) }
}

pub fn freebox_url(ip: &str, port: u16, code_reseau: &str, touche: &str) -> String { format!("http://{ip}:{port}/pub/remote_control?code={code_reseau}&key={touche}") }

pub async fn freebox(ip: &str, code_reseau: Option<&str>, touche: &str) -> Result<(), String> { freebox_port(ip, 80, code_reseau, touche).await }
async fn freebox_port(ip: &str, port: u16, code_reseau: Option<&str>, touche: &str) -> Result<(), String> {
    let c = code_reseau.filter(|c| !c.is_empty() && c.len() <= 12 && c.bytes().all(|b| b.is_ascii_digit()))
        .ok_or("code télécommande réseau manquant : Freebox Player > Réglages > Système > Informations, recopie le code dans le profil")?;
    ok(client()?.get(freebox_url(ip, port, c, touche)).send().await)
}

pub fn orange_url(ip: &str, port: u16, touche: &str) -> String { format!("http://{ip}:{port}/remoteControl/cmd?operation=01&key={touche}&mode=0") }

pub async fn orange(ip: &str, touche: &str) -> Result<(), String> { orange_port(ip, 8080, touche).await }
async fn orange_port(ip: &str, port: u16, touche: &str) -> Result<(), String> {
    if touche.is_empty() || !touche.bytes().all(|b| b.is_ascii_digit()) { return Err("code de touche invalide".into()); }
    ok(client()?.get(orange_url(ip, port, touche)).send().await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    fn faux() -> (u16, Arc<Mutex<Vec<String>>>) {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let vu = Arc::new(Mutex::new(vec![]));
        let v2 = vu.clone();
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut b = [0u8; 2048];
                let n = s.read(&mut b).unwrap_or(0);
                v2.lock().unwrap().push(String::from_utf8_lossy(&b[..n]).to_string());
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
            }
        });
        (port, vu)
    }
    fn rt() -> tokio::runtime::Runtime { tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap() }

    #[test]
    fn tables() {
        assert_eq!(code("vol_up", 12).unwrap(), "vol_inc");
        assert_eq!(code("vol_up", 13).unwrap(), "115");
        assert_eq!(code("power", 13).unwrap(), "116");
        assert!(code("netflix", 12).is_err() && code("home", 13).is_err());
        assert!(supported(12).len() > 30 && supported(13).len() > 25);
    }

    #[test]
    fn freebox_requete() {
        let (p, vu) = faux();
        rt().block_on(freebox_port("127.0.0.1", p, Some("12345678"), "vol_inc")).unwrap();
        assert!(vu.lock().unwrap()[0].starts_with("GET /pub/remote_control?code=12345678&key=vol_inc HTTP/1.1"));
        assert!(rt().block_on(freebox_port("127.0.0.1", p, None, "power")).is_err(), "sans code");
        assert!(rt().block_on(freebox_port("127.0.0.1", p, Some("12&x=1"), "power")).is_err(), "code hostile");
    }

    #[test]
    fn orange_requete() {
        let (p, vu) = faux();
        rt().block_on(orange_port("127.0.0.1", p, "116")).unwrap();
        assert!(vu.lock().unwrap()[0].starts_with("GET /remoteControl/cmd?operation=01&key=116&mode=0 HTTP/1.1"));
        assert!(rt().block_on(orange_port("127.0.0.1", p, "1&mode=9")).is_err(), "touche hostile");
    }
}
