//! Wake-on-LAN : réveille une TV en veille (la touche marche/arrêt ne peut pas, la TV n'écoute plus).
use tokio::net::UdpSocket;

pub fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    let v: Vec<u8> = s.trim().split([':', '-']).filter_map(|p| u8::from_str_radix(p, 16).ok()).collect();
    <[u8; 6]>::try_from(v).map_err(|_| "adresse MAC invalide (ex. AA:BB:CC:DD:EE:FF)".to_string())
}

pub async fn wake(mac: &str) -> Result<(), String> {
    let m = parse_mac(mac)?;
    let mut p = vec![0xFFu8; 6];
    for _ in 0..16 { p.extend_from_slice(&m); }
    let s = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    s.set_broadcast(true).map_err(|e| e.to_string())?;
    s.send_to(&p, "255.255.255.255:9").await.map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn mac() {
        assert_eq!(super::parse_mac("aa:BB:cc:dd:ee:01").unwrap(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 1]);
        assert!(super::parse_mac("zz").is_err());
        assert!(super::parse_mac("aa:bb").is_err());
    }
}
