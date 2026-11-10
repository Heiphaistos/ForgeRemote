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

/// Marques infrarouge : l'émission se fait côté téléphone (interface), pas ici.
const IR: [&str; 3] = ["ir_samsung", "ir_lg", "ir_sony"];

fn brand_idx(b: &str) -> Result<usize, String> {
    keys::BRANDS.iter().position(|x| *x == b).ok_or_else(|| "marque non gérée".to_string())
}

/// Jamais le jeton ni la clé vers l'interface.
fn vue(t: &Tv) -> Value {
    json!({"id":t.id,"name":t.name,"brand":t.brand,"ip":t.ip,"paired":t.token.is_some(),"ir":IR.contains(&t.brand.as_str()),"wol":t.mac.is_some()})
}

#[tauri::command]
fn list_tvs(st: St) -> Vec<Value> { st.lock().unwrap().tvs.iter().map(vue).collect() }

#[tauri::command]
fn add_tv(st: St, name: String, brand: String, ip: String, secret: Option<String>, mac: Option<String>) -> Result<Value, String> {
    let ir = IR.contains(&brand.as_str());
    let ip = if ir { String::new() } else { brand_idx(&brand)?; valider_ip(&ip)?.to_string() };
    let name = name.trim().chars().take(40).collect::<String>();
    if name.is_empty() { return Err("nom requis".into()); }
    let mac = mac.map(|m| m.trim().to_string()).filter(|m| !m.is_empty());
    if let Some(m) = &mac { wol::parse_mac(m)?; }
    let token = secret.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis().to_string();
    let tv = Tv { id, name, brand, ip, token, mac };
    let mut s = st.lock().unwrap();
    s.tvs.push(tv.clone());
    s.save()?;
    Ok(vue(&tv))
}

#[tauri::command]
fn remove_tv(st: St, id: String) -> Result<(), String> {
    let mut s = st.lock().unwrap();
    s.tvs.retain(|t| t.id != id);
    s.save()
}

async fn envoyer(st: &St<'_>, id: &str, key: Option<&str>) -> Result<(), String> {
    let tv = st.lock().unwrap().get(id)?; // verrou relâché avant tout await
    if IR.contains(&tv.brand.as_str()) { return Err("TV infrarouge : envoi depuis le téléphone uniquement".into()); }
    let b = brand_idx(&tv.brand)?;
    let code = key.map(|k| keys::code(k, b)).transpose()?;
    let tok = tv.token.as_deref();
    // Marche/arrêt avec MAC connue : on réveille d'abord (une TV en veille n'écoute plus le réseau).
    let reveil = key == Some("power") && match &tv.mac { Some(m) => wol::wake(m).await.is_ok(), None => false };
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
async fn scan_tvs() -> Result<Vec<Value>, String> { ssdp::scan().await }

#[tauri::command]
async fn check_update() -> Result<Option<Value>, String> { update::check().await }

#[tauri::command]
async fn install_update(app: AppHandle, url: String, name: String, sums: String) -> Result<(), String> {
    // Version portable (non installée par l'installeur) : on ne lance pas d'installeur à sa place.
    if tauri::utils::platform::bundle_type() != Some(tauri::utils::config::BundleType::Nsis) {
        return Err("version portable : télécharge la nouvelle version sur la page de la release".into());
    }
    update::install(&url, &name, &sums).await?;
    app.exit(0); // l'installeur silencieux prend le relais
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            app.manage(Mutex::new(Store::load(dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_tvs, add_tv, remove_tv, send_key, pair, scan_tvs, check_update, install_update])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de ForgeRemote");
}
