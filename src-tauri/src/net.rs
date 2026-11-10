//! Connexion WebSocket vers une TV du réseau local.
//! Les TV présentent un certificat auto-signé : on l'accepte, mais seulement pour une TV
//! de la liste blanche dont l'IP a été validée privée (voir store::valider_ip).
use std::{sync::Arc, time::Duration};
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

#[derive(Debug)]
struct Lan(Arc<CryptoProvider>);

impl ServerCertVerifier for Lan {
    fn verify_server_cert(&self, _: &CertificateDer, _: &[CertificateDer], _: &ServerName, _: &[u8], _: UnixTime) -> Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        verify_tls12_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn verify_tls13_signature(&self, m: &[u8], c: &CertificateDer, d: &DigitallySignedStruct) -> Result<HandshakeSignatureValid, Error> {
        verify_tls13_signature(m, c, d, &self.0.signature_verification_algorithms)
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.0.signature_verification_algorithms.supported_schemes()
    }
}

pub async fn ws(url: &str, secs: u64) -> Result<Ws, String> {
    let p = Arc::new(ring::default_provider());
    let cfg = ClientConfig::builder_with_provider(p.clone())
        .with_safe_default_protocol_versions()
        .map_err(|e| e.to_string())?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(Lan(p)))
        .with_no_client_auth();
    let c = connect_async_tls_with_config(url, None, false, Some(Connector::Rustls(Arc::new(cfg))));
    let (s, _) = tokio::time::timeout(Duration::from_secs(secs), c)
        .await
        .map_err(|_| "TV injoignable (délai dépassé)".to_string())?
        .map_err(|e| format!("connexion refusée : {e}"))?;
    Ok(s)
}
