mod keys;
mod lg;
mod net;
mod panasonic;
mod samsung;
mod store;

use serde_json::{json, Value};
use std::sync::Mutex;
use store::{valider_ip, Store, Tv};
use tauri::{Manager, State};

type St<'a> = State<'a, Mutex<Store>>;

fn brand_n(b: &str) -> Result<u8, String> {
    match b { "samsung" => Ok(1), "lg" => Ok(2), "panasonic" => Ok(3), _ => Err("marque non gérée".into()) }
}

/// Jamais le jeton vers l'interface.
fn vue(t: &Tv) -> Value { json!({"id":t.id,"name":t.name,"brand":t.brand,"ip":t.ip,"paired":t.token.is_some()}) }

#[tauri::command]
fn list_tvs(st: St) -> Vec<Value> { st.lock().unwrap().tvs.iter().map(vue).collect() }

#[tauri::command]
fn add_tv(st: St, name: String, brand: String, ip: String) -> Result<Value, String> {
    brand_n(&brand)?;
    let ip = valider_ip(&ip)?.to_string();
    let name = name.trim().chars().take(40).collect::<String>();
    if name.is_empty() { return Err("nom requis".into()); }
    let id = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_err(|e| e.to_string())?.as_millis().to_string();
    let tv = Tv { id, name, brand, ip, token: None };
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
    let b = brand_n(&tv.brand)?;
    let code = key.map(|k| keys::code(k, b)).transpose()?;
    let tok = tv.token.as_deref();
    let nouveau = match b {
        1 => samsung::send(&tv.ip, tok, code).await?,
        2 => lg::send(&tv.ip, tok, code).await?,
        _ => { if let Some(c) = code { panasonic::send(&tv.ip, c).await?; } None }
    };
    if let Some(n) = nouveau.filter(|n| Some(n.as_str()) != tok) {
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
            app.manage(Mutex::new(Store::load(dir)));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![list_tvs, add_tv, remove_tv, send_key, pair])
        .run(tauri::generate_context!())
        .expect("erreur au lancement de ForgeRemote");
}
