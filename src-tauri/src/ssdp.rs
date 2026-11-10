//! Recherche SSDP : liste seulement les TV de marques gérées. Rien n'est contrôlé : c'est
//! l'utilisateur qui choisit d'ajouter (liste blanche).
use serde_json::{json, Value};
use std::{collections::HashMap, net::Ipv4Addr, time::Duration};
use tokio::net::UdpSocket;

fn marque(t: &str) -> Option<&'static str> {
    let t = t.to_lowercase();
    [("samsung", "samsung"), ("webos", "lg"), ("lge", "lg"), ("panasonic", "panasonic"), ("viera", "panasonic"),
     ("roku", "roku"), ("philips", "philips"), ("bravia", "sony"), ("sony", "sony")]
        .iter().find(|(k, _)| t.contains(k)).map(|(_, b)| *b)
}

pub async fn scan() -> Result<Vec<Value>, String> {
    let s = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    let req = "M-SEARCH * HTTP/1.1\r\nHOST: 239.255.255.250:1900\r\nMAN: \"ssdp:discover\"\r\nMX: 2\r\nST: ssdp:all\r\n\r\n";
    s.send_to(req.as_bytes(), "239.255.255.250:1900").await.map_err(|e| e.to_string())?;
    let mut trouvees: HashMap<Ipv4Addr, &'static str> = HashMap::new();
    let mut buf = [0u8; 2048];
    let fin = tokio::time::Instant::now() + Duration::from_secs(3);
    while let Ok(Ok((n, from))) = tokio::time::timeout_at(fin, s.recv_from(&mut buf)).await {
        if let (std::net::IpAddr::V4(ip), Some(b)) = (from.ip(), marque(&String::from_utf8_lossy(&buf[..n]))) {
            if ip.is_private() { trouvees.insert(ip, b); }
        }
    }
    let mut v: Vec<_> = trouvees.into_iter().map(|(ip, b)| json!({"ip": ip.to_string(), "brand": b})).collect();
    v.sort_by_key(|x| x["ip"].as_str().unwrap_or("").to_string());
    Ok(v)
}

#[cfg(test)]
mod tests {
    #[test]
    fn marques() {
        assert_eq!(super::marque("SERVER: Linux UPnP/1.0 Samsung"), Some("samsung"));
        assert_eq!(super::marque("SERVER: WebOS/4.0 UPnP"), Some("lg"));
        assert_eq!(super::marque("SERVER: Linux Printer HP"), None);
    }
}
