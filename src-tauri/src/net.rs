//! Connexion WebSocket vers une TV du réseau local.
//! Les TV présentent un certificat auto-signé : on l'accepte, mais seulement pour une TV
//! de la liste blanche dont l'IP a été validée privée (voir store::valider_ip).
//! Empreinte épinglée à la première connexion (TOFU) : si le certificat d'une TV change ensuite
//! (autre appareil à cette adresse, usurpation sur le réseau local), la connexion est refusée.
use sha2::{Digest, Sha256};
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex, OnceLock}, time::Duration};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    connect_async_tls_with_config, Connector, MaybeTlsStream, WebSocketStream,
};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::{ring, verify_tls12_signature, verify_tls13_signature, CryptoProvider},
    pki_types::{CertificateDer, ServerName, UnixTime},
    ClientConfig, DigitallySignedStruct, Error, SignatureScheme,
};

pub type Ws = WebSocketStream<MaybeTlsStream<TcpStream>>;

static PINS: Mutex<Option<HashMap<String, String>>> = Mutex::new(None);
static FICHIER: OnceLock<PathBuf> = OnceLock::new();

/// Charge les empreintes mémorisées (pins.json dans le dossier de données).
pub fn init(dir: PathBuf) {
    let f = dir.join("pins.json");
    let m: HashMap<String, String> = std::fs::read_to_string(&f).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
    *PINS.lock().unwrap() = Some(m);
    let _ = FICHIER.set(f);
}

fn sauver(m: &HashMap<String, String>) {
    if let Some(f) = FICHIER.get() {
        let tmp = f.with_extension("tmp");
        if serde_json::to_string(m).ok().and_then(|t| std::fs::write(&tmp, t).ok()).is_some() { let _ = std::fs::rename(&tmp, f); }
    }
}

/// Oublie l'empreinte d'une TV (certificat légitimement changé : réinitialisation, mise à jour majeure).
pub fn oublier(ip: &str) {
    let mut g = PINS.lock().unwrap();
    if let Some(m) = g.as_mut() { if m.remove(ip).is_some() { sauver(m); } }
}

/// Premier certificat vu pour cet hôte : mémorisé ; ensuite il doit être identique.
fn controler(hote: &str, empreinte: &str) -> Result<(), Error> {
    let mut g = PINS.lock().unwrap();
    let m = g.get_or_insert_with(HashMap::new);
    match m.get(hote) {
        Some(p) if p != empreinte => Err(Error::General("EMPREINTE".into())),
        Some(_) => Ok(()),
        None => { m.insert(hote.to_string(), empreinte.to_string()); sauver(m); Ok(()) }
    }
}

#[derive(Debug)]
struct Lan { prov: Arc<CryptoProvider>, hote: Option<String> }

impl ServerCertVerifier for Lan {
    fn verify_server_cert(&self, cert: &CertificateDer, _: &[CertificateDer], _: &ServerName, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, Error> {
        if let Some(h) = &self.hote {
            let fp: String = Sha256::digest(cert.as_ref()).iter().map(|b| format!("{b:02x}")).collect();
            controler(h, &fp)?;
        }
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(m, c, d, &self.prov.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(m, c, d, &self.prov.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.prov.signature_verification_algorithms.supported_schemes()
    }
}

/// Connexion à une TV de la liste blanche : empreinte du certificat épinglée (TOFU).
pub async fn ws(url: &str, secs: u64) -> Result<Ws, String> { connecter(url, secs, true).await }

/// Sondage d'une adresse pas encore enregistrée (détection) : aucune empreinte mémorisée.
pub async fn ws_libre(url: &str, secs: u64) -> Result<Ws, String> { connecter(url, secs, false).await }

async fn connecter(url: &str, secs: u64, epingler: bool) -> Result<Ws, String> {
    let hote = epingler.then(|| url.split("://").nth(1).unwrap_or("").split(['/', ':', '?']).next().unwrap_or("").to_string()).filter(|h| !h.is_empty());
    let p = Arc::new(ring::default_provider());
    let cfg = ClientConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Lan { prov: p, hote }))
        .with_no_client_auth();
    let c = connect_async_tls_with_config(url, None, false, Some(Connector::Rustls(Arc::new(cfg))));
    let (s, _) = tokio::time::timeout(Duration::from_secs(secs), c)
        .await
        .map_err(|_| "TV injoignable (délai dépassé)".to_string())?
        .map_err(|e| {
            let t = e.to_string();
            if t.contains("EMPREINTE") { "Le certificat de la TV a changé (TV réinitialisée, ou autre appareil à cette adresse ?). Si c'est bien ta TV : Modifier > Enregistrer pour oublier l'ancienne empreinte.".to_string() }
            else { format!("connexion refusée : {t}") }
        })?;
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empreinte_epinglee_puis_refusee_si_elle_change() {
        assert!(controler("test-hote-1", "aa").is_ok(), "première vue : mémorisée");
        assert!(controler("test-hote-1", "aa").is_ok(), "identique : accepté");
        assert!(controler("test-hote-1", "bb").is_err(), "différente : refusée");
        oublier("test-hote-1");
        assert!(controler("test-hote-1", "bb").is_ok(), "après oubli : nouvelle empreinte mémorisée");
        assert!(controler("test-hote-2", "bb").is_ok(), "autre hôte indépendant");
    }

    /// Test manuel contre un vrai serveur TLS local (voir tools : serveur Python, deux certificats). Lancé avec --ignored.
    #[test]
    #[ignore]
    fn tls_reel() {
        init(std::path::PathBuf::from(std::env::var("PIN_DIR").unwrap()));
        let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
        let r = rt.block_on(ws(&format!("wss://127.0.0.1:{}/", std::env::var("PORT").unwrap()), 3));
        let msg = r.err().unwrap_or_default();
        println!("RESULTAT: {msg}");
        match std::env::var("ATTENDU").unwrap().as_str() { "change" => assert!(msg.contains("certificat de la TV a changé")), _ => assert!(!msg.contains("certificat de la TV a changé")) }
    }
}
