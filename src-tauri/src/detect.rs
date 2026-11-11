//! Détection automatique d'une TV par sondage de ports connus (sans multicast : marche sur Android
//! et derrière les adaptateurs virtuels qui font échouer SSDP). On ne fait qu'IDENTIFIER :
//! l'ajout à la liste blanche reste un geste de l'utilisateur.
use crate::net::ws_libre as ws;
use futures_util::{future::join_all, stream, FutureExt, StreamExt};
use serde_json::{json, Value};
use std::{net::Ipv4Addr, time::Duration};
use tokio::{net::{TcpStream, UdpSocket}, time::timeout};

type Id = Option<String>; // Some(nom lisible) si la marque répond

fn client() -> Option<reqwest::Client> {
    reqwest::Client::builder().redirect(reqwest::redirect::Policy::none()).timeout(Duration::from_millis(1800)).build().ok()
}

async fn get(c: &reqwest::Client, url: String) -> Option<String> {
    let r = c.get(url).send().await.ok()?;
    if r.status().is_success() { r.text().await.ok() } else { None }
}

/// Noms renvoyés par les TV : « 65&quot; Neo QLED » -> « 65" Neo QLED ».
fn decoder(s: &str) -> String {
    s.replace("&quot;", "\"").replace("&#34;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&")
}

fn entre<'a>(s: &'a str, a: &str, b: &str) -> Option<&'a str> {
    let i = s.find(a)? + a.len();
    Some(&s[i..i + s[i..].find(b)?])
}

async fn samsung(c: reqwest::Client, ip: Ipv4Addr) -> Id {
    let v: Value = serde_json::from_str(&get(&c, format!("http://{ip}:8001/api/v2/")).await?).ok()?;
    Some(decoder(v["device"]["name"].as_str().or(v["name"].as_str()).unwrap_or("Samsung")))
}
async fn roku(c: reqwest::Client, ip: Ipv4Addr) -> Id {
    let t = get(&c, format!("http://{ip}:8060/query/device-info")).await?;
    Some(decoder(entre(&t, "<friendly-device-name>", "<").or(entre(&t, "<model-name>", "<")).unwrap_or("Roku")))
}
async fn panasonic(c: reqwest::Client, ip: Ipv4Addr) -> Id {
    let t = get(&c, format!("http://{ip}:55000/nrc/ddd.xml")).await?;
    t.to_lowercase().contains("panasonic").then(|| entre(&t, "<friendlyName>", "<").unwrap_or("Panasonic Viera").to_string())
}
async fn philips(c: reqwest::Client, ip: Ipv4Addr) -> Id {
    let v: Value = serde_json::from_str(&get(&c, format!("http://{ip}:1925/6/system")).await?).ok()?;
    Some(v["name"].as_str().unwrap_or("Philips").to_string())
}
async fn sony(c: reqwest::Client, ip: Ipv4Addr) -> Id {
    let r = c.post(format!("http://{ip}/sony/system"))
        .json(&json!({"method":"getSystemInformation","id":1,"params":[],"version":"1.0"})).send().await.ok()?;
    let v: Value = r.json().await.ok()?;
    v["result"][0]["model"].as_str().map(|m| format!("Sony {m}"))
}
async fn lg(ip: Ipv4Addr) -> Id {
    let ok = ws(&format!("wss://{ip}:3001"), 2).await.is_ok() || ws(&format!("ws://{ip}:3000"), 2).await.is_ok();
    ok.then(|| "LG webOS".to_string())
}

async fn xiaomi(ip: Ipv4Addr) -> Id { crate::extra::xiaomi_vivante(&ip.to_string()).await.then(|| "Xiaomi Mi TV".to_string()) }
async fn vizio(ip: Ipv4Addr) -> Id { crate::extra::vizio_vivante(&ip.to_string()).await.then(|| "Vizio SmartCast".to_string()) }

/// Adresse MAC Wi-Fi lue dans l'API de la TV allumée (marche aussi sur Android, où la table ARP est interdite).
/// Samsung : GET :8001/api/v2/ -> device.wifiMac ; Roku : GET :8060/query/device-info -> <wifi-mac>.
pub async fn mac_via_api(brand: &str, ip: &str) -> Option<String> {
    let c = client()?;
    let m = match brand {
        "samsung" => {
            let v: Value = serde_json::from_str(&get(&c, format!("http://{ip}:8001/api/v2/")).await?).ok()?;
            v["device"]["wifiMac"].as_str()?.to_string()
        }
        "roku" => entre(&get(&c, format!("http://{ip}:8060/query/device-info")).await?, "<wifi-mac>", "<")?.to_string(),
        _ => return None,
    };
    crate::wol::parse_mac(&m).ok().map(|b| b.iter().map(|x| format!("{x:02X}")).collect::<Vec<_>>().join(":"))
}

/// Identifie la marque de la TV à cette IP. None = rien de reconnu.
pub async fn probe(ip: Ipv4Addr) -> Option<Value> {
    let c = client()?;
    let tests: Vec<(&str, futures_util::future::BoxFuture<Id>)> = vec![
        ("samsung", samsung(c.clone(), ip).boxed()),
        ("roku", roku(c.clone(), ip).boxed()),
        ("panasonic", panasonic(c.clone(), ip).boxed()),
        ("philips", philips(c.clone(), ip).boxed()),
        ("sony", sony(c.clone(), ip).boxed()),
        ("lg", lg(ip).boxed()),
        ("xiaomi", xiaomi(ip).boxed()),
        ("vizio", vizio(ip).boxed()),
    ];
    let (marques, futs): (Vec<_>, Vec<_>) = tests.into_iter().unzip();
    let res = join_all(futs).await;
    marques.into_iter().zip(res).find_map(|(m, n)| n.map(|n| json!({"ip": ip.to_string(), "brand": m, "name": n})))
}

async fn ouvert(ip: Ipv4Addr, port: u16) -> bool {
    timeout(Duration::from_millis(600), TcpStream::connect((ip, port))).await.is_ok_and(|r| r.is_ok())
}

/// Adresse IPv4 locale de la route par défaut (aucun paquet envoyé).
pub async fn ip_locale() -> Result<Ipv4Addr, String> {
    let s = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    s.connect("8.8.8.8:80").await.map_err(|_| "pas de réseau".to_string())?;
    match s.local_addr().map_err(|e| e.to_string())?.ip() {
        std::net::IpAddr::V4(a) if a.is_private() => Ok(a),
        _ => Err("pas de réseau local privé (Wi-Fi ?)".into()),
    }
}

/// Parcourt le /24 local : ports ouverts puis identification. Réseau local uniquement.
pub async fn scan_sous_reseau() -> Result<Vec<Value>, String> {
    let moi = ip_locale().await?;
    let o = moi.octets();
    let hotes = (1..255u8).map(move |h| Ipv4Addr::new(o[0], o[1], o[2], h)).filter(move |a| *a != moi);
    let mut v: Vec<Value> = stream::iter(hotes)
        .map(|ip| async move {
            let sonde = [8001u16, 8060, 3001, 3000, 55000, 1925, 80, 6095, 7345, 9000];
            let r = join_all(sonde.iter().map(|p| ouvert(ip, *p))).await;
            if r.iter().any(|x| *x) { probe(ip).await } else { None }
        })
        .buffer_unordered(96)
        .filter_map(|x| async move { x })
        .collect().await;
    v.sort_by_key(|x| x["ip"].as_str().unwrap_or("").to_string());
    Ok(v)
}

#[cfg(test)]
mod tests {
    #[test]
    fn detecte_une_fausse_roku_et_ignore_le_reste() {
        use std::io::{Read, Write};
        let l = std::net::TcpListener::bind("127.0.0.1:8060").expect("port 8060 libre");
        std::thread::spawn(move || {
            for mut s in l.incoming().flatten() {
                let mut b = [0u8; 512];
                let _ = s.read(&mut b);
                let body = "<device-info><friendly-device-name>Salon Roku</friendly-device-name></device-info>";
                let _ = write!(s, "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len());
            }
        });
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(super::probe("127.0.0.1".parse().unwrap())).expect("Roku reconnue");
        assert_eq!((r["brand"].as_str(), r["name"].as_str()), (Some("roku"), Some("Salon Roku")));
    }

    #[test]
    fn noms_decodes() {
        assert_eq!(super::decoder("65&quot; Neo QLED &amp; co"), "65\" Neo QLED & co");
    }

    #[test]
    fn extraction_xml() {
        assert_eq!(super::entre("<a><friendly-device-name>Salon</friendly-device-name>", "<friendly-device-name>", "<"), Some("Salon"));
        assert_eq!(super::entre("rien", "<x>", "<"), None);
    }
}
