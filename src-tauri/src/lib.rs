mod detect;
mod http;
mod keys;
mod lg;
mod net;
mod panasonic;
mod samsung;
mod sony;
mod ssdp;
mod store;
mod update;
mod wol;

use serde_json::{json, Value};
use std::sync::Mutex;
use store::{valider_ip, Store, Tv};
use tauri::{AppHandle, Manager, State};

type St<'a> = State<'a, Mutex<Store>>;

/// Profil envoyé depuis le téléphone (infrarouge ou Bluetooth), jamais par le réseau.
fn is_local(b: &str) -> bool { b == "bt" || is_ir(b) }

/// Marques infrarouge (`ir_<marque>`, codes dans src/ir-db.js) : l'émission se fait côté téléphone, pas ici.
fn is_ir(b: &str) -> bool {
    b.strip_prefix("ir_").is_some_and(|m| !m.is_empty() && m.len() <= 30 && m.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()))
}

fn brand_idx(b: &str) -> Result<usize, String> {
    keys::BRANDS.iter().position(|x| *x == b).ok_or_else(|| "marque non gérée".to_string())
}

/// Jamais le jeton ni la clé vers l'interface.
fn vue(t: &Tv) -> Value {
    json!({"id":t.id,"name":t.name,"brand":t.brand,"ip":t.ip,"paired":t.token.is_some(),"ir":is_local(&t.brand),"wol":t.mac.is_some(),"mac":t.mac,"power":t.power})
}

#[cfg(test)]
mod tests {
    #[test]
    fn marques_ir() {
        assert!(super::is_ir("ir_samsung") && super::is_ir("ir_lg") && super::is_ir("ir_continentaledison"));
        assert!(super::is_local("bt") && super::is_local("ir_lg") && !super::is_local("btx") && !super::is_local("lg"));
        assert!(!super::is_ir("samsung") && !super::is_ir("ir_") && !super::is_ir("ir_Sam sung") && !super::is_ir("ir_../x"));
    }
}

#[tauri::command]
fn list_tvs(st: St) -> Vec<Value> { st.lock().unwrap().tvs.iter().map(vue).collect() }

#[tauri::command]
fn add_tv(st: St, name: String, brand: String, ip: String, secret: Option<String>, mac: Option<String>, power: Option<String>) -> Result<Value, String> {
    let ir = is_local(&brand);
    let ip = if ir { String::new() } else { brand_idx(&brand)?; valider_ip(&ip)?.to_string() };
    let name = name.trim().chars().take(40).collect::<String>();
    if name.is_empty() { return Err("nom requis".into()); }
    let mac = mac.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    if let Some(m) = &mac { wol::parse_mac(m)?; }
    let power = store::power_valide(power)?;
    let token = secret.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis().to_string();
    let tv = Tv { id, name, brand, ip, token, mac, power };
    let mut s = st.lock().unwrap();
    s.tvs.push(tv.clone());
    s.save()?;
    Ok(vue(&tv))
}

/// Modifie un profil existant (nom, marque, IP, MAC, clé). Le jeton d'appairage est gardé
/// sauf si la marque change ou si une nouvelle clé est fournie.
#[tauri::command]
fn update_tv(st: St, id: String, name: String, brand: String, ip: String, secret: Option<String>, mac: Option<String>, power: Option<String>) -> Result<Value, String> {
    let ir = is_local(&brand);
    let ip = if ir { String::new() } else { brand_idx(&brand)?; valider_ip(&ip)?.to_string() };
    let name = name.trim().chars().take(40).collect::<String>();
    if name.is_empty() { return Err("nom requis".into()); }
    let mac = mac.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    if let Some(m) = &mac { wol::parse_mac(m)?; }
    let power = store::power_valide(power)?;
    let secret = secret.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let mut s = st.lock().unwrap();
    let t = s.tvs.iter_mut().find(|t| t.id == id).ok_or("TV introuvable")?;
    if t.brand != brand { t.token = None; }
    if secret.is_some() { t.token = secret; }
    (t.name, t.brand, t.ip, t.mac, t.power) = (name, brand, ip, mac, power);
    let v = vue(t);
    s.save()?;
    Ok(v)
}

/// Mode auto : identifie la marque de la TV à cette IP (IP privée obligatoire).
#[tauri::command]
async fn detect_tv(ip: String) -> Result<Option<Value>, String> {
    Ok(detect::probe(valider_ip(&ip)?).await)
}

#[tauri::command]
fn remove_tv(st: St, id: String) -> Result<(), String> {
    let mut s = st.lock().unwrap();
    s.tvs.retain(|t| t.id != id);
    s.save()
}

/// Cherche la MAC de la TV dans la table ARP et la mémorise dans son profil.
async fn apprendre_mac(st: &St<'_>, id: &str, ip: &str) -> Option<String> {
    let m = wol::lookup_mac(ip).await?;
    let mut s = st.lock().unwrap();
    if let Some(t) = s.tvs.iter_mut().find(|t| t.id == id && t.mac.is_none()) { t.mac = Some(m.clone()); let _ = s.save(); }
    Some(m)
}

/// Appelée après l'ajout d'une TV allumée : retient sa MAC pour pouvoir l'allumer depuis la veille.
#[tauri::command]
async fn learn_mac(st: St<'_>, id: String) -> Result<bool, String> {
    let tv = st.lock().unwrap().get(&id)?;
    if is_local(&tv.brand) || tv.mac.is_some() { return Ok(tv.mac.is_some()); }
    Ok(apprendre_mac(&st, &id, &tv.ip).await.is_some())
}

async fn envoyer(st: &St<'_>, id: &str, key: Option<&str>) -> Result<(), String> {
    let tv = st.lock().unwrap().get(id)?; // verrou relâché avant tout await
    if is_local(&tv.brand) { return Err("TV infrarouge ou Bluetooth : envoi depuis le téléphone uniquement".into()); }
    let b = brand_idx(&tv.brand)?;
    let code = key.map(|k| keys::code(k, b)).transpose()?;
    let tok = tv.token.as_deref();
    // MAC apprise automatiquement tant que la TV est allumée : permet de la réveiller plus tard sans rien saisir.
    let mac = match &tv.mac { Some(m) => Some(m.clone()), None => apprendre_mac(st, id, &tv.ip).await };
    // Marche/arrêt avec MAC connue : on réveille d'abord (une TV en veille n'écoute plus le réseau).
    let reveil = key == Some("power") && match &mac { Some(m) => wol::wake(m, &tv.ip).await.is_ok(), None => false };
    let res: Result<Option<String>, String> = match (b, code) {
        (0, c) => samsung::send(&tv.ip, tok, c).await,
        (1, c) => lg::send(&tv.ip, tok, c).await,
        (_, None) => Ok(None),
        (2, Some(c)) => panasonic::send(&tv.ip, c).await.map(|_| None),
        (3, Some(c)) => http::roku(&tv.ip, c).await.map(|_| None),
        (4, Some(c)) => http::philips(&tv.ip, c).await.map(|_| None),
        (_, Some(c)) => sony::send(&tv.ip, tok, c).await.map(|_| None),
    };
    let nouveau = match res {
        Ok(n) => n,
        Err(_) if reveil => None, // TV en veille : le paquet de réveil suffit
        Err(e) => return Err(e),
    };
    if let (Some(n), true) = (nouveau.filter(|n| Some(n.as_str()) != tok), b < 2) {
        let mut s = st.lock().unwrap();
        if let Some(t) = s.tvs.iter_mut().find(|t| t.id == id) { t.token = Some(n); }
        s.save()?;
    }
    Ok(())
}

#[tauri::command]
async fn send_key(st: St<'_>, id: String, key: String) -> Result<(), String> { envoyer(&st, &id, Some(&key)).await }

#[tauri::command]
async fn pair(st: St<'_>, id: String) -> Result<(), String> { envoyer(&st, &id, None).await }

#[tauri::command]
async fn scan_tvs() -> Result<Vec<Value>, String> {
    // Sondage du sous-réseau (fiable) + SSDP (complément) en parallèle, fusionnés par IP.
    let (sous, upnp) = futures_util::future::join(detect::scan_sous_reseau(), ssdp::scan()).await;
    let mut v = sous.unwrap_or_default();
    for d in upnp.unwrap_or_default() {
        if !v.iter().any(|x| x["ip"] == d["ip"]) { v.push(json!({"ip": d["ip"], "brand": d["brand"], "name": d["brand"]})); }
    }
    if v.is_empty() { detect::ip_locale().await?; } // erreur claire si pas de réseau local
    Ok(v)
}

#[tauri::command]
async fn check_update() -> Result<Option<Value>, String> { update::check().await }

#[tauri::command]
async fn install_update(app: AppHandle, url: String, name: String, sums: String) -> Result<(), String> {
    update::install(&url, &name, &sums).await?;
    app.exit(0); // l'installeur silencieux prend le relais
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            update::nettoyer();
            app.manage(Mutex::new(Store::load(dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_tvs, add_tv, update_tv, detect_tv, remove_tv, send_key, learn_mac, pair, scan_tvs, check_update, install_update])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de ForgeRemote");
}
