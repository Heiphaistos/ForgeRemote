//! Android TV / Google TV : protocole « Remote v2 » (appairage TLS sur 6467 avec code à 6 chiffres hexadécimaux,
//! puis télécommande sur 6466). Le téléphone/PC présente un certificat client RSA généré une fois et mémorisé.
//! Messages protobuf écrits à la main (varint + champs). Conforme aux sources publiques (androidtvremote2, polo) et
//! à la structure observée dans l'appli Roku (code d'appairage hexadécimal à somme de contrôle, empreinte du
//! certificat de la TV mémorisée). Vérifié contre une fausse TV, PAS sur une vraie.
use rsa::{pkcs8::EncodePrivateKey, traits::PublicKeyParts, RsaPrivateKey};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    ClientConfig, DigitallySignedStruct, Error, SignatureScheme,
};
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex, OnceLock}, time::Duration};
use tokio::{io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt}, net::TcpStream, time::timeout};
use tokio_rustls::{client::TlsStream, TlsConnector};

static DIR: OnceLock<PathBuf> = OnceLock::new();
pub fn init(dir: PathBuf) { let _ = DIR.set(dir); }

type Flux = TlsStream<TcpStream>;
/// Appairage en cours : ip -> (flux TLS ouvert, module et exposant RSA de la TV).
static EN_COURS: Mutex<Option<HashMap<String, (Flux, Vec<u8>, Vec<u8>)>>> = Mutex::new(None);

// ---------- protobuf minimal ----------
fn varint(mut v: u64, o: &mut Vec<u8>) { loop { let b = (v & 0x7f) as u8; v >>= 7; if v == 0 { o.push(b); break; } o.push(b | 0x80); } }
fn champ_v(n: u32, v: u64) -> Vec<u8> { let mut o = vec![]; varint((n as u64) << 3, &mut o); varint(v, &mut o); o }
fn champ_b(n: u32, b: &[u8]) -> Vec<u8> { let mut o = vec![]; varint(((n as u64) << 3) | 2, &mut o); varint(b.len() as u64, &mut o); o.extend_from_slice(b); o }
fn concat(p: &[Vec<u8>]) -> Vec<u8> { p.iter().flatten().copied().collect() }

#[derive(Debug, PartialEq)]
enum Val { N(u64), B(Vec<u8>) }

fn lire_varint(b: &[u8], i: &mut usize) -> Option<u64> {
    let (mut v, mut s) = (0u64, 0u32);
    loop { let x = *b.get(*i)?; *i += 1; if s > 63 { return None; } v |= ((x & 0x7f) as u64) << s; if x & 0x80 == 0 { return Some(v); } s += 7; }
}

fn champs(b: &[u8]) -> Option<Vec<(u32, Val)>> {
    let (mut i, mut r) = (0, vec![]);
    while i < b.len() {
        let t = lire_varint(b, &mut i)?;
        match t & 7 {
            0 => r.push(((t >> 3) as u32, Val::N(lire_varint(b, &mut i)?))),
            2 => { let l = lire_varint(b, &mut i)? as usize; r.push(((t >> 3) as u32, Val::B(b.get(i..i.checked_add(l)?)?.to_vec()))); i += l; }
            _ => return None,
        }
    }
    Some(r)
}
fn num(c: &[(u32, Val)], n: u32) -> Option<u64> { c.iter().find_map(|(k, v)| match v { Val::N(x) if *k == n => Some(*x), _ => None }) }
fn octets(c: &[(u32, Val)], n: u32) -> Option<Vec<u8>> { c.iter().find_map(|(k, v)| match v { Val::B(x) if *k == n => Some(x.clone()), _ => None }) }

async fn ecrire<S: AsyncWrite + Unpin>(s: &mut S, m: &[u8]) -> Result<(), String> {
    let mut o = vec![]; varint(m.len() as u64, &mut o); o.extend_from_slice(m);
    s.write_all(&o).await.map_err(|e| e.to_string())?; s.flush().await.map_err(|e| e.to_string())
}

async fn lire<S: AsyncRead + Unpin>(s: &mut S) -> Result<Vec<u8>, String> {
    timeout(Duration::from_secs(8), async {
        let (mut l, mut sh) = (0u64, 0u32);
        loop { let b = s.read_u8().await.map_err(|e| e.to_string())?; l |= ((b & 0x7f) as u64) << sh; if b & 0x80 == 0 { break; } sh += 7; if sh > 28 { return Err("trame invalide".to_string()); } }
        if l > 65536 { return Err("trame trop grande".into()); }
        let mut buf = vec![0u8; l as usize];
        s.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        Ok(buf)
    }).await.map_err(|_| "TV : pas de réponse".to_string())?
}

// ---------- identité du client (certificat RSA auto-signé, généré une fois) ----------
fn identite() -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>, Vec<u8>, Vec<u8>), String> {
    let dir = DIR.get().ok_or("dossier de données non initialisé")?;
    let (kf, cf) = (dir.join("atv-key.der"), dir.join("atv-cert.der"));
    let cle_der = match (std::fs::read(&kf), std::fs::read(&cf)) {
        (Ok(k), Ok(_)) => k,
        _ => {
            let k = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).map_err(|e| e.to_string())?;
            let der = k.to_pkcs8_der().map_err(|e| e.to_string())?.as_bytes().to_vec();
            let kp = rcgen::KeyPair::try_from(der.as_slice()).map_err(|e| e.to_string())?;
            let mut p = rcgen::CertificateParams::new(vec![]).map_err(|e| e.to_string())?;
            p.distinguished_name.push(rcgen::DnType::CommonName, "ForgeRemote");
            let cert = p.self_signed(&kp).map_err(|e| e.to_string())?;
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            std::fs::write(&kf, &der).map_err(|e| e.to_string())?;
            std::fs::write(&cf, cert.der().as_ref()).map_err(|e| e.to_string())?;
            der
        }
    };
    let cert = std::fs::read(&cf).map_err(|e| e.to_string())?;
    let k = <RsaPrivateKey as rsa::pkcs8::DecodePrivateKey>::from_pkcs8_der(&cle_der).map_err(|e| e.to_string())?;
    Ok((CertificateDer::from(cert), PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(cle_der)), k.n().to_bytes_be(), k.e().to_bytes_be()))
}

/// (module, exposant) RSA du certificat de la TV, en gros-boutiste sans zéro de tête.
fn cle_rsa_cert(der: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    use x509_parser::{prelude::*, public_key::PublicKey};
    let (_, c) = X509Certificate::from_der(der).map_err(|e| e.to_string())?;
    match c.public_key().parsed().map_err(|e| e.to_string())? {
        PublicKey::RSA(r) => Ok((sans_zero(r.modulus), sans_zero(r.exponent))),
        _ => Err("certificat de la TV : clé non RSA".into()),
    }
}
fn sans_zero(b: &[u8]) -> Vec<u8> { let i = b.iter().position(|x| *x != 0).unwrap_or(b.len()); b[i..].to_vec() }

#[derive(Debug)]
struct Tout(Arc<CryptoProvider>);
impl ServerCertVerifier for Tout {
    fn verify_server_cert(&self, _: &CertificateDer, _: &[CertificateDer], _: &ServerName, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, Error> { Ok(ServerCertVerified::assertion()) }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> { verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms) }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> { verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms) }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> { self.0.signature_verification_algorithms.supported_schemes() }
}

/// Connexion TLS avec certificat client ; l'empreinte du certificat de la TV est épinglée (TOFU).
async fn tls(ip: &str, port: u16) -> Result<(Flux, Vec<u8>), String> {
    let (cert, cle, _, _) = identite()?;
    let p = Arc::new(ring::default_provider());
    let cfg = ClientConfig::builder_with_provider(p.clone()).with_safe_default_protocol_versions().map_err(|e| e.to_string())?
        .dangerous().with_custom_certificate_verifier(Arc::new(Tout(p))).with_client_auth_cert(vec![cert], cle).map_err(|e| e.to_string())?;
    let addr: std::net::IpAddr = ip.parse().map_err(|_| "IP invalide".to_string())?;
    let tcp = timeout(Duration::from_secs(4), TcpStream::connect((addr, port))).await.map_err(|_| "TV injoignable (délai dépassé)".to_string())?.map_err(|e| format!("TV injoignable : {e}"))?;
    let s = timeout(Duration::from_secs(5), TlsConnector::from(Arc::new(cfg)).connect(ServerName::IpAddress(addr.into()), tcp)).await
        .map_err(|_| "TLS : délai dépassé".to_string())?.map_err(|e| format!("TLS refusé : {e}"))?;
    let der = s.get_ref().1.peer_certificates().and_then(|c| c.first()).map(|c| c.as_ref().to_vec()).ok_or("la TV n'a pas présenté de certificat")?;
    crate::net::epingler(&format!("{ip}:atv"), &der)?;
    Ok((s, der))
}

// ---------- appairage ----------
const SERVICE: &str = "androidtvremote2";
fn msg_appairage(interne: Vec<u8>) -> Vec<u8> { concat(&[champ_v(1, 2), champ_v(2, 200), interne]) }
fn encodage() -> Vec<u8> { concat(&[champ_v(1, 3), champ_v(2, 6)]) } // hexadécimal, 6 symboles

async fn attendre_ok<S: AsyncRead + Unpin>(s: &mut S) -> Result<Vec<(u32, Val)>, String> {
    let c = champs(&lire(s).await?).ok_or("réponse de la TV illisible")?;
    if num(&c, 2) != Some(200) { return Err("appairage refusé par la TV".into()); }
    Ok(c)
}

/// 1re étape : la TV affiche un code (6 caractères hexadécimaux) ; la connexion reste ouverte pour la 2e étape.
pub async fn appairage_debut(ip: &str) -> Result<(), String> { appairage_debut_port(ip, 6467).await }
async fn appairage_debut_port(ip: &str, port: u16) -> Result<(), String> {
    let (mut s, der) = tls(ip, port).await?;
    let (m, e) = cle_rsa_cert(&der)?;
    ecrire(&mut s, &msg_appairage(champ_b(10, &concat(&[champ_b(1, SERVICE.as_bytes()), champ_b(2, b"ForgeRemote")])))).await?;
    attendre_ok(&mut s).await?;
    ecrire(&mut s, &msg_appairage(champ_b(20, &concat(&[champ_b(1, &encodage()), champ_v(3, 1)])))).await?;
    attendre_ok(&mut s).await?;
    ecrire(&mut s, &msg_appairage(champ_b(30, &concat(&[champ_b(1, &encodage()), champ_v(2, 1)])))).await?;
    attendre_ok(&mut s).await?;
    EN_COURS.lock().unwrap().get_or_insert_with(HashMap::new).insert(ip.to_string(), (s, m, e));
    Ok(())
}

/// Secret = SHA-256(module client, exposant client, module TV, exposant TV, code sans son 1er octet) ; le 1er octet du code doit égaler le 1er octet du secret.
fn secret(client: (&[u8], &[u8]), tv: (&[u8], &[u8]), code: &[u8]) -> Result<Vec<u8>, String> {
    let mut h = Sha256::new();
    for p in [client.0, client.1, tv.0, tv.1, &code[1..]] { h.update(p); }
    let r = h.finalize().to_vec();
    if r[0] != code[0] { return Err("Code incorrect (somme de contrôle)".into()); }
    Ok(r)
}

pub fn code_octets(code: &str) -> Result<Vec<u8>, String> {
    let c = code.trim();
    if c.len() != 6 || !c.bytes().all(|b| b.is_ascii_hexdigit()) { return Err("le code comporte 6 caractères (0-9, A-F)".into()); }
    (0..6).step_by(2).map(|i| u8::from_str_radix(&c[i..i + 2], 16).map_err(|e| e.to_string())).collect()
}

/// 2e étape : envoie le secret ; Ok = appairé (l'identité est celle de l'application, rien d'autre à mémoriser).
pub async fn appairage_fin(ip: &str, code: &str) -> Result<(), String> {
    let c = code_octets(code)?;
    let (mut s, tm, te) = EN_COURS.lock().unwrap().as_mut().and_then(|m| m.remove(ip)).ok_or("clique d'abord sur « Afficher le code sur la TV »")?;
    let (_, _, cm, ce) = identite()?;
    let sec = secret((&cm, &ce), (&tm, &te), &c)?;
    ecrire(&mut s, &msg_appairage(champ_b(40, &champ_b(1, &sec)))).await?;
    attendre_ok(&mut s).await?;
    Ok(())
}

// ---------- télécommande ----------
/// Codes de touches Android (KeyEvent) d'après la table de l'appli Roku et la documentation Android.
pub fn touche_android(id: &str) -> Option<u32> {
    Some(match id {
        "power" => 26, "home" => 3, "back" => 4, "menu" => 82, "up" => 19, "down" => 20, "left" => 21, "right" => 22, "ok" => 23,
        "vol_up" => 24, "vol_down" => 25, "mute" => 164, "ch_up" => 166, "ch_down" => 167, "play" => 126, "pause" => 127, "rew" => 89, "ff" => 90,
        "stop" => 86, "info" => 165, "guide" => 172, "source" => 178, "tv" => 170, "search" => 84, "subtitle" => 175,
        "red" => 183, "green" => 184, "yellow" => 185, "blue" => 186,
        "0" => 7, "1" => 8, "2" => 9, "3" => 10, "4" => 11, "5" => 12, "6" => 13, "7" => 14, "8" => 15, "9" => 16,
        _ => return None,
    })
}

/// Paquets Android des applis courantes (lancement par lien market://launch?id=).
pub fn paquet(nom: &str) -> Option<&'static str> {
    [("plex", "com.plexapp.android"), ("netflix", "com.netflix.ninja"), ("youtube", "com.google.android.youtube.tv"), ("prime", "com.amazon.amazonvideo.livingroom"),
     ("disney", "com.disney.disneyplus"), ("spotify", "com.spotify.tv.android"), ("twitch", "tv.twitch.android.app"), ("kodi", "org.xbmc.kodi"), ("vlc", "org.videolan.vlc")]
        .iter().find(|(n, _)| nom.contains(n)).map(|(_, p)| *p)
}

fn config_client() -> Vec<u8> {
    let info = concat(&[champ_b(1, b"ForgeRemote"), champ_b(2, b"ForgeRemote"), champ_v(3, 1), champ_b(4, b"1"), champ_b(5, b"org.heiphaistos.forgeremote"), champ_b(6, b"1.0.0")]);
    champ_b(1, &concat(&[champ_v(1, 622), champ_b(2, &info)]))
}

/// Envoie une touche (`KEY:<code>`) ou lance une appli (`APP:<nom>`) : une session complète par commande.
pub async fn envoyer(ip: &str, code: &str) -> Result<(), String> { envoyer_port(ip, 6466, code).await }
async fn envoyer_port(ip: &str, port: u16, code: &str) -> Result<(), String> {
    let (mut s, _) = tls(ip, port).await.map_err(|e| format!("{e} (TV non appairée ?)"))?;
    let action = if let Some(k) = code.strip_prefix("KEY:") {
        champ_b(10, &concat(&[champ_v(1, k.parse::<u64>().map_err(|_| "code de touche invalide")?), champ_v(2, 3)]))
    } else if let Some(n) = code.strip_prefix("APP:") {
        let p = paquet(n).ok_or_else(|| format!("application « {n} » inconnue pour Android TV (connues : Plex, Netflix, YouTube, Prime, Disney+, Spotify, Twitch, Kodi, VLC)"))?;
        champ_b(90, &champ_b(1, format!("market://launch?id={p}").as_bytes()))
    } else { return Err("commande Android TV invalide".into()) };
    // Poignée de main : la TV envoie sa configuration, on répond, puis on s'active ; les pings reçoivent une réponse.
    let mut prete = false;
    for _ in 0..12 {
        let c = champs(&lire(&mut s).await?).ok_or("message de la TV illisible")?;
        if octets(&c, 1).is_some() { ecrire(&mut s, &config_client()).await?; }
        else if octets(&c, 2).is_some() { ecrire(&mut s, &champ_b(2, &champ_v(1, 622))).await?; prete = true; break; }
        else if let Some(p) = octets(&c, 8) { let v = champs(&p).and_then(|x| num(&x, 1)).unwrap_or(0); ecrire(&mut s, &champ_b(9, &champ_v(1, v))).await?; }
        else if octets(&c, 3).is_some() { return Err("la TV a refusé la connexion (appairage requis)".into()); }
    }
    if !prete { return Err("la TV n'a pas terminé la connexion".into()); }
    ecrire(&mut s, &action).await?;
    tokio::time::sleep(Duration::from_millis(150)).await;
    let _ = s.shutdown().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protobuf_aller_retour() {
        let m = concat(&[champ_v(1, 2), champ_v(2, 200), champ_b(10, b"abc")]);
        let c = champs(&m).unwrap();
        assert_eq!((num(&c, 1), num(&c, 2), octets(&c, 10)), (Some(2), Some(200), Some(b"abc".to_vec())));
        assert_eq!(champ_v(1, 300), vec![0x08, 0xac, 0x02], "varint 300");
        assert!(champs(&[0x0a, 0x05, 1]).is_none(), "longueur trop grande");
    }

    #[test]
    fn code_et_secret() {
        assert_eq!(code_octets("1A2b3C").unwrap(), vec![0x1a, 0x2b, 0x3c]);
        assert!(code_octets("12345").is_err() && code_octets("12345G").is_err());
        // le secret dépend de toutes les entrées et le 1er octet du code sert de somme de contrôle
        let mut h = Sha256::new();
        for p in [&[1u8][..], &[2], &[3], &[4], &[0x2b, 0x3c]] { h.update(p); }
        let bon = h.finalize()[0];
        assert!(secret((&[1], &[2]), (&[3], &[4]), &[bon, 0x2b, 0x3c]).is_ok());
        assert!(secret((&[1], &[2]), (&[3], &[4]), &[bon ^ 1, 0x2b, 0x3c]).is_err());
    }

    #[test]
    fn touches_et_applis() {
        assert_eq!((touche_android("ok"), touche_android("5"), touche_android("zzz")), (Some(23), Some(12), None));
        assert_eq!(paquet("disney+"), Some("com.disney.disneyplus"));
        assert_eq!(paquet("inconnue"), None);
        assert_eq!(sans_zero(&[0, 0, 1, 0, 1]), vec![1, 0, 1]);
    }

    /// Fausse TV Android : vrai TLS, mêmes messages que le protocole ; vérifie la suite complète côté TV.
    #[test]
    fn fausse_tv_appairage_puis_touche() {
        use tokio_rustls::TlsAcceptor;
        let dir = std::env::temp_dir().join(format!("atv-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(dir.clone());
        // certificat de la fausse TV (RSA)
        let k = RsaPrivateKey::new(&mut rsa::rand_core::OsRng, 2048).unwrap();
        let der = k.to_pkcs8_der().unwrap().as_bytes().to_vec();
        let kp = rcgen::KeyPair::try_from(der.as_slice()).unwrap();
        let cert = rcgen::CertificateParams::new(vec![]).unwrap().self_signed(&kp).unwrap();
        let (sm, se) = (k.n().to_bytes_be(), k.e().to_bytes_be());
        let p = Arc::new(ring::default_provider());
        let cfg = rustls::ServerConfig::builder_with_provider(p).with_safe_default_protocol_versions().unwrap().with_no_client_auth()
            .with_single_cert(vec![CertificateDer::from(cert.der().to_vec())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(der))).unwrap();
        let acc = TlsAcceptor::from(Arc::new(cfg));
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        rt.block_on(async {
            let l1 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let l2 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let (p1, p2) = (l1.local_addr().unwrap().port(), l2.local_addr().unwrap().port());
            let (a1, a2) = (acc.clone(), acc);
            let (_, _, cm, ce) = identite().unwrap();
            // --- TV, côté appairage ---
            let tv1 = tokio::spawn(async move {
                let (t, _) = l1.accept().await.unwrap();
                let mut s = a1.accept(t).await.unwrap();
                let c = champs(&lire(&mut s).await.unwrap()).unwrap();
                let req = champs(&octets(&c, 10).expect("PairingRequest")).unwrap();
                assert_eq!((octets(&req, 1), octets(&req, 2)), (Some(SERVICE.as_bytes().to_vec()), Some(b"ForgeRemote".to_vec())));
                ecrire(&mut s, &msg_appairage(champ_b(11, b""))).await.unwrap();
                let c = champs(&lire(&mut s).await.unwrap()).unwrap();
                assert!(octets(&c, 20).is_some(), "PairingOption");
                ecrire(&mut s, &msg_appairage(champ_b(20, b""))).await.unwrap();
                let c = champs(&lire(&mut s).await.unwrap()).unwrap();
                let conf = champs(&octets(&c, 30).expect("PairingConfiguration")).unwrap();
                assert_eq!(num(&conf, 2), Some(1), "rôle INPUT");
                ecrire(&mut s, &msg_appairage(champ_b(31, b""))).await.unwrap();
                let c = champs(&lire(&mut s).await.unwrap()).unwrap();
                let sec = champs(&octets(&c, 40).expect("PairingSecret")).unwrap();
                ecrire(&mut s, &msg_appairage(champ_b(41, b""))).await.unwrap();
                octets(&sec, 1).unwrap()
            });
            appairage_debut_port("127.0.0.1", p1).await.unwrap();
            // le code affiché par la TV : 1er octet = 1er octet du secret attendu (calculé ici comme la TV)
            let mut h = Sha256::new();
            for p in [&cm[..], &ce[..], &sm[..], &se[..], &[0x22, 0x33]] { h.update(p); }
            let attendu = h.finalize().to_vec();
            appairage_fin("127.0.0.1", &format!("{:02x}2233", attendu[0])).await.unwrap();
            assert_eq!(tv1.await.unwrap(), attendu, "secret reçu par la TV");
            // mauvais code : refusé localement
            let acc3 = TlsAcceptor::from(Arc::new(rustls::ServerConfig::builder_with_provider(Arc::new(ring::default_provider())).with_safe_default_protocol_versions().unwrap().with_no_client_auth()
                .with_single_cert(vec![CertificateDer::from(cert.der().to_vec())], PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(k.to_pkcs8_der().unwrap().as_bytes().to_vec()))).unwrap()));
            let l3 = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let p3 = l3.local_addr().unwrap().port();
            let tv3 = tokio::spawn(async move {
                let (t, _) = l3.accept().await.unwrap(); let mut s = acc3.accept(t).await.unwrap();
                for r in [11u32, 20, 31] { let _ = lire(&mut s).await.unwrap(); ecrire(&mut s, &msg_appairage(champ_b(r, b""))).await.unwrap(); }
            });
            appairage_debut_port("127.0.0.1", p3).await.unwrap();
            assert!(appairage_fin("127.0.0.1", &format!("{:02x}2233", attendu[0] ^ 1)).await.is_err(), "mauvais code refusé");
            tv3.await.unwrap();
            // --- TV, côté télécommande ---
            let tv2 = tokio::spawn(async move {
                let (t, _) = l2.accept().await.unwrap();
                let mut s = a2.accept(t).await.unwrap();
                ecrire(&mut s, &champ_b(1, &concat(&[champ_v(1, 622)]))).await.unwrap();
                let c = champs(&lire(&mut s).await.unwrap()).unwrap();
                assert!(octets(&c, 1).is_some(), "configure du client");
                ecrire(&mut s, &champ_b(8, &champ_v(1, 7))).await.unwrap(); // ping
                ecrire(&mut s, &champ_b(2, &champ_v(1, 622))).await.unwrap(); // set_active
                let mut recu = vec![];
                for _ in 0..3 { if let Ok(m) = lire(&mut s).await { recu.push(champs(&m).unwrap()); } }
                recu
            });
            envoyer_port("127.0.0.1", p2, "KEY:23").await.unwrap();
            let recu = tv2.await.unwrap();
            let ping = recu.iter().find(|c| octets(c, 9).is_some()).expect("réponse au ping");
            assert_eq!(num(&champs(&octets(ping, 9).unwrap()).unwrap(), 1), Some(7));
            assert!(recu.iter().any(|c| octets(c, 2).is_some()), "set_active du client");
            let touche = recu.iter().find_map(|c| octets(c, 10)).expect("touche injectée");
            let t = champs(&touche).unwrap();
            assert_eq!((num(&t, 1), num(&t, 2)), (Some(23), Some(3)));
        });
        let _ = std::fs::remove_dir_all(&dir);
    }
}
