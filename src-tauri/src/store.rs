//! Liste blanche des TV : seules celles ajoutées ici sont jamais contactées.
use serde::{Deserialize, Serialize};
use std::{net::{IpAddr, Ipv4Addr}, path::PathBuf};

#[derive(Serialize, Deserialize, Clone)]
pub struct Tv {
    pub id: String,
    pub name: String,
    pub brand: String, // samsung | lg | panasonic
    pub ip: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub mac: Option<String>,
}

pub struct Store { path: PathBuf, pub tvs: Vec<Tv> }

/// IP privée uniquement : empêche d'envoyer quoi que ce soit hors du réseau local.
pub fn valider_ip(s: &str) -> Result<Ipv4Addr, String> {
    match s.trim().parse::<IpAddr>() {
        Ok(IpAddr::V4(a)) if a.is_private() || a.is_loopback() => Ok(a),
        _ => Err("IP invalide : adresse du réseau local attendue (ex. 192.168.1.20)".into()),
    }
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let path = dir.join("tvs.json");
        let tvs = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        Store { path, tvs }
    }
    pub fn save(&self) -> Result<(), String> {
        if let Some(d) = self.path.parent() { std::fs::create_dir_all(d).map_err(|e| e.to_string())?; }
        let tmp = self.path.with_extension("tmp"); // écriture atomique
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.tvs).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        std::fs::rename(&tmp, &self.path).map_err(|e| e.to_string())
    }
    pub fn get(&self, id: &str) -> Result<Tv, String> {
        self.tvs.iter().find(|t| t.id == id).cloned().ok_or_else(|| "TV non autorisée".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::valider_ip;
    #[test]
    fn ip_privee_seulement() {
        assert!(valider_ip("192.168.1.20").is_ok());
        assert!(valider_ip("10.0.0.5").is_ok());
        assert!(valider_ip("8.8.8.8").is_err());
        assert!(valider_ip("169.254.169.254").is_err());
        assert!(valider_ip("evil.com").is_err());
    }
}
