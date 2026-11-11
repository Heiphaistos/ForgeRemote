//! Samsung « legacy » (téléviseurs d'avant 2014 : séries B à F) : TCP 55000, paquets à longueurs d'un octet.
//! Format relevé dans la source publique de samsungctl (remote_legacy.py) : chaînes en base64 préfixées de leur
//! longueur et d'un 0x00 ; réponses 64 00 01 00 (accepté), 64 00 00 00 (refusé), 0a.. (attente de l'accord sur la TV),
//! 65.. (annulé), 00 00 00 00 (touche acceptée). Vérifié contre une fausse TV, PAS sur un vrai téléviseur.
use base64::{engine::general_purpose::STANDARD, Engine};
use std::time::Duration;
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}, net::TcpStream, time::timeout};

/// base64 de la chaîne, précédé de (longueur, 0x00).
fn ser(s: &[u8]) -> Vec<u8> { let b = STANDARD.encode(s).into_bytes(); [&[b.len() as u8, 0][..], &b].concat() }
/// Chaîne brute (déjà formée) précédée de (longueur, 0x00).
fn ser_brut(s: &[u8]) -> Vec<u8> { [&[s.len() as u8, 0][..], s].concat() }

pub fn paquet_auth() -> Vec<u8> {
    let payload = [&[0x64u8, 0][..], &ser(b"ForgeRemote"), &ser(b"forgeremote"), &ser(b"ForgeRemote")].concat();
    [&[0u8, 0, 0][..], &ser_brut(&payload)].concat()
}

pub fn paquet_touche(touche: &str) -> Vec<u8> {
    let payload = [&[0u8, 0, 0][..], &ser(touche.as_bytes())].concat();
    [&[0u8, 0, 0][..], &ser_brut(&payload)].concat()
}

enum Rep { Accepte, Attente }

async fn lire_reponse<S: AsyncRead + Unpin>(s: &mut S) -> Result<Rep, String> {
    let mut h = [0u8; 3];
    s.read_exact(&mut h).await.map_err(|e| e.to_string())?;
    let mut nom = vec![0u8; u16::from_le_bytes([h[1], h[2]]) as usize];
    s.read_exact(&mut nom).await.map_err(|e| e.to_string())?;
    let mut l = [0u8; 2];
    s.read_exact(&mut l).await.map_err(|e| e.to_string())?;
    let n = u16::from_le_bytes(l) as usize;
    if n == 0 || n > 256 { return Err("TV : connexion fermée".into()); }
    let mut r = vec![0u8; n];
    s.read_exact(&mut r).await.map_err(|e| e.to_string())?;
    match r.as_slice() {
        [0x64, 0, 1, 0] | [0, 0, 0, 0] => Ok(Rep::Accepte),
        [0x64, 0, 0, 0] => Err("accès refusé sur la TV".into()),
        [0x65, ..] => Err("autorisation annulée sur la TV".into()),
        [0x0a, ..] => Ok(Rep::Attente),
        _ => Err("réponse inattendue de la TV".into()),
    }
}

async fn session<S: AsyncRead + AsyncWrite + Unpin>(s: &mut S, touche: Option<&str>) -> Result<(), String> {
    s.write_all(&paquet_auth()).await.map_err(|e| e.to_string())?;
    // « 0a » = la TV demande d'accepter l'appareil à l'écran (30 s)
    timeout(Duration::from_secs(30), async {
        loop { if let Rep::Accepte = lire_reponse(s).await? { return Ok::<(), String>(()); } }
    }).await.map_err(|_| "pas de confirmation sur la TV (accepte l'appareil à l'écran)".to_string())??;
    if let Some(t) = touche {
        s.write_all(&paquet_touche(t)).await.map_err(|e| e.to_string())?;
        timeout(Duration::from_secs(5), lire_reponse(s)).await.map_err(|_| "TV : pas de réponse".to_string())??;
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    Ok(())
}

/// touche=None : appairage seul (la TV affiche une demande d'accord).
pub async fn envoyer(ip: &str, touche: Option<&str>) -> Result<(), String> { envoyer_port(ip, 55000, touche).await }

async fn envoyer_port(ip: &str, port: u16, touche: Option<&str>) -> Result<(), String> {
    if let Some(t) = touche { if t.is_empty() || t.len() > 40 || !t.bytes().all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_') { return Err("touche invalide".into()); } }
    let mut s = timeout(Duration::from_secs(4), TcpStream::connect((ip, port))).await.map_err(|_| "TV injoignable (délai dépassé)".to_string())?.map_err(|e| format!("TV injoignable : {e}"))?;
    session(&mut s, touche).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paquets_conformes_a_samsungctl() {
        // touche : 00 00 00 | len 00 | (00 00 00 | len 00 | base64)
        let p = paquet_touche("KEY_VOLUP"); // base64 = S0VZX1ZPTFVQ (12 octets)
        let b64 = b"S0VZX1ZPTFVQ";
        let interne = [&[0u8, 0, 0, 12, 0][..], b64].concat();
        assert_eq!(p, [&[0u8, 0, 0, interne.len() as u8, 0][..], &interne].concat());
        let a = paquet_auth();
        assert_eq!(&a[..3], &[0, 0, 0]);
        assert_eq!(&a[5..7], &[0x64, 0], "début du payload d'authentification");
        assert_eq!(a[3] as usize, a.len() - 5, "longueur du payload");
    }

    /// Fausse TV : demande d'accord (0a) puis accord, puis accepte une touche.
    #[test]
    fn fausse_tv_accord_puis_touche() {
        let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        let tv = std::thread::spawn(move || {
            use std::io::{Read, Write};
            let (mut s, _) = l.accept().unwrap();
            let rep = |s: &mut std::net::TcpStream, r: &[u8]| { let nom = b"TV"; let mut m = vec![0u8]; m.extend((nom.len() as u16).to_le_bytes()); m.extend(nom); m.extend((r.len() as u16).to_le_bytes()); m.extend(r); s.write_all(&m).unwrap(); };
            let mut a = vec![0u8; paquet_auth().len()];
            s.read_exact(&mut a).unwrap();
            assert_eq!(a, paquet_auth());
            rep(&mut s, &[0x0a, 0, 0]);
            rep(&mut s, &[0x64, 0, 1, 0]);
            let mut k = vec![0u8; paquet_touche("KEY_MUTE").len()];
            s.read_exact(&mut k).unwrap();
            rep(&mut s, &[0, 0, 0, 0]);
            k
        });
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(envoyer_port("127.0.0.1", port, Some("KEY_MUTE"))).unwrap();
        assert_eq!(tv.join().unwrap(), paquet_touche("KEY_MUTE"));
        assert!(rt.block_on(envoyer_port("127.0.0.1", port, Some("rm -rf"))).is_err());
    }
}
