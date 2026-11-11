//! Wake-on-LAN : réveille une TV en veille (la touche marche/arrêt ne peut pas, la TV n'écoute plus).
use tokio::net::UdpSocket;

pub fn parse_mac(s: &str) -> Result<[u8; 6], String> {
    let v: Vec<u8> = s.trim().split([':', '-']).filter_map(|p| u8::from_str_radix(p, 16).ok()).collect();
    <[u8; 6]>::try_from(v).map_err(|_| "adresse MAC invalide (ex. AA:BB:CC:DD:EE:FF)".to_string())
}

/// Réveille la TV : paquet magique en diffusion générale ET en diffusion du /24 de la TV (si connue),
/// ports 9 et 7 (la diffusion générale sort parfois par la mauvaise carte réseau sous Windows).
pub async fn wake(mac: &str, ip: &str) -> Result<(), String> {
    let m = parse_mac(mac)?;
    let mut p = vec![0xFFu8; 6];
    for _ in 0..16 { p.extend_from_slice(&m); }
    let s = UdpSocket::bind("0.0.0.0:0").await.map_err(|e| e.to_string())?;
    s.set_broadcast(true).map_err(|e| e.to_string())?;
    let mut cibles = vec!["255.255.255.255".to_string()];
    if let Ok(a) = ip.parse::<std::net::Ipv4Addr>() {
        let o = a.octets();
        cibles.push(format!("{}.{}.{}.255", o[0], o[1], o[2]));
    }
    let mut ok = false;
    for _ in 0..3 {
        for c in &cibles { for port in [9, 7] { ok |= s.send_to(&p, format!("{c}:{port}")).await.is_ok(); } }
    }
    if ok { Ok(()) } else { Err("envoi du réveil impossible".into()) }
}

/// Adresse MAC d'une TV déjà vue sur le réseau (table ARP) : évite de la saisir à la main.
/// Android ≥ 10 interdit cette lecture (le téléphone allume en infrarouge) : renvoie None.
pub async fn lookup_mac(ip: &str) -> Option<String> {
    let a: std::net::Ipv4Addr = ip.parse().ok()?;
    // Un paquet vers la TV remplit l'entrée ARP si elle manquait.
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0").await { let _ = s.send_to(&[0], (a, 9)).await; }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let txt = tauri::async_runtime::spawn_blocking(arp_table).await.ok()?;
    txt.lines().find_map(|l| {
        let mut w = l.split_whitespace();
        let (first, rest) = (w.next()?, w.collect::<Vec<_>>());
        let mac = rest.iter().find_map(|x| parse_mac(x).ok())?;
        (first == ip && mac != [0; 6] && mac != [0xff; 6])
            .then(|| mac.iter().map(|b| format!("{b:02X}")).collect::<Vec<_>>().join(":"))
    })
}

#[cfg(windows)]
fn arp_table() -> String {
    use std::os::windows::process::CommandExt;
    std::process::Command::new("arp").arg("-a").creation_flags(0x0800_0000).output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default()
}
#[cfg(not(windows))]
fn arp_table() -> String { std::fs::read_to_string("/proc/net/arp").unwrap_or_default() }

#[cfg(test)]
mod tests {
    #[test]
    fn mac() {
        assert_eq!(super::parse_mac("aa:BB:cc:dd:ee:01").unwrap(), [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 1]);
        assert!(super::parse_mac("zz").is_err());
        assert!(super::parse_mac("aa:bb").is_err());
    }
}

#[cfg(test)]
mod lookup_test {
    /// Réseau réel : lance avec `WOL_TEST_IP=192.168.1.105 cargo test lookup -- --nocapture`.
    #[test]
    fn lookup() {
        let Ok(ip) = std::env::var("WOL_TEST_IP") else { return };
        let m = tauri::async_runtime::block_on(super::lookup_mac(&ip));
        println!("MAC de {ip} = {m:?}");
        assert!(m.is_some());
    }
}
