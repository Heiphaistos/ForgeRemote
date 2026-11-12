import { irDispo, irKeyFor, irMarques, irPowerWifi, irSend } from "./ir.js";
import { initLook } from "./look.js";
import { initScenes } from "./scenes.js";
import { BT_KEYS, btAdresse, btAppareils, btSend, btText, btVisible } from "./bt.js";

const invoke = window.__TAURI__.core.invoke;
const $ = (s) => document.querySelector(s);
const sel = $("#tv"), msg = $("#msg"), dlg = $("#dlg"), brandSel = $("#f-brand");
let tvs = [], editing = null, grpOn = false;

const say = (t, err) => { msg.textContent = t; msg.className = err ? "err" : ""; };
const cur = () => tvs.find((t) => t.id === sel.value);
const store = (k, v) => { try { v === undefined ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch {} };
const load = (k) => { try { return localStorage.getItem(k); } catch { return null; } };
const NEEDS_PAIR = ["samsung", "lg"];
$("#f-ir-brand").replaceChildren(...irMarques().map((x) => new Option(x.name, x.id)));
$("#ir-group").replaceChildren(...irMarques().map((m) => new Option(`${m.name} (IR)`, m.id)));
const AIDE_RESEAU = "Vérifie que la TV est allumée, sur le même Wi-Fi, et que l'autorisation « réseau local / appareils à proximité » est accordée à l'app (Android).";

// Groupe : TV cochées qui reçoivent chaque touche en même temps.
const grpIds = () => { try { return JSON.parse(load("grp") ?? "[]"); } catch { return []; } };
const cibles = () => {
  const g = grpOn ? tvs.filter((t) => grpIds().includes(t.id)) : [];
  return g.length ? g : cur() ? [cur()] : [];
};

function buildGroup() {
  const ids = grpIds();
  $("#group").replaceChildren(...tvs.map((t) => {
    const l = document.createElement("label"), c = document.createElement("input");
    c.type = "checkbox"; c.checked = ids.includes(t.id);
    c.onchange = () => { store("grp", JSON.stringify([...document.querySelectorAll("#group input")].flatMap((x, i) => (x.checked ? [tvs[i].id] : [])))); majTouches(); };
    l.append(c, ` ${t.name}`);
    return l;
  }));
}

async function refresh(keep) {
  tvs = await invoke("list_tvs");
  sel.replaceChildren(...tvs.map((t) => new Option(`${t.name} (${t.brand})`, t.id)));
  if (keep) sel.value = keep;
  buildGroup();
  if (!tvs.length) say(irPowerWifi("samsung") ? "Ajoute ta TV : choisis sa marque « (IR) » pour l'allumer sans appairage ni IP" : "Ajoute ta première TV avec le bouton Ajouter");
  else if (!cur().paired && NEEDS_PAIR.includes(cur().brand)) say("TV non appairée : appuie sur une touche et accepte sur la TV");
  else say("");
  store("tv", sel.value);
  majTouches();
}

// Ce que chaque profil sait faire : Wi-Fi = table de la marque (Rust), IR = base de codes, Bluetooth = table HID.
// Un profil peut COMBINER plusieurs modes : Wi-Fi (marque + IP), infrarouge (émetteur du téléphone), Bluetooth.
// Chaque touche prend le premier mode qui la gère, avec repli sur le suivant si l'envoi échoue.
const base = (tv) => tv.brand.replace(/^ir_/, "");
const estLg = (tv) => base(tv) === "lg" && !!tv.ip;
const peutWifi = (tv, k) => (k.startsWith("app:") ? tv.apps : k.startsWith("src:") ? estLg(tv) : !!tv.keys?.includes(k));
/** Marque IR utilisable : profil IR, IR ajouté au profil, ou (téléphone avec émetteur) la même marque que le Wi-Fi. */
const irDe = (tv) => (tv.brand.startsWith("ir_") ? tv.brand : tv.irBrand || (tv.brand !== "bt" && irDispo() ? `ir_${base(tv)}` : null));
const okWifi = (tv, k) => tv.brand !== "bt" && !!tv.ip && peutWifi(tv, k);
/** Profil infrarouge sans IP : un bouton d'appli peut retrouver l'IP de la TV tout seul (Samsung, LG, Roku). */
const peutDecouvrir = (tv, k) => k.startsWith("app:") && !tv.ip && tv.brand.startsWith("ir_") && ["samsung", "lg", "roku"].includes(base(tv));
const okIr = (tv, k) => { const b = irDe(tv); return !!b && irKeyFor(b, k) !== null; };
const okBt = (tv, k) => (tv.brand === "bt" || tv.bt) && BT_KEYS.includes(k);
const support = (tv, k) => okWifi(tv, k) || okIr(tv, k) || okBt(tv, k) || peutDecouvrir(tv, k);
/** Grise les touches que la/les TV ciblée(s) ne gèrent pas (au lieu d'une erreur au clic). */
function majTouches() {
  const cs = cibles();
  $("#srcbox").hidden = !(cur() && estLg(cur()));
  document.querySelectorAll("[data-k]").forEach((b) => {
    const ok = !cs.length || cs.some((tv) => support(tv, b.dataset.k));
    b.disabled = !ok; b.classList.toggle("indispo", !ok);
    if (!ok) b.title = "Non disponible pour ce profil (ajoute un mode : Modifier > Combiner plusieurs modes)"; else if (b.title.startsWith("Non dispo")) b.removeAttribute("title");
  });
}

/** Cherche la TV sur le réseau et enregistre son IP dans le profil (une seule TV de cette marque doit répondre). */
async function decouvrir(tv) {
  say("Recherche de la TV sur le Wi-Fi (environ 8 s)…");
  const l = (await invoke("scan_tvs")).filter((d) => d.brand === base(tv));
  if (!l.length) throw new Error("TV introuvable sur le Wi-Fi (allumée ? même réseau ? autorisation « réseau local » accordée ?) : saisis son IP dans Modifier");
  if (l.length > 1) throw new Error(`Plusieurs TV de cette marque trouvées (${l.map((d) => d.ip).join(", ")}) : saisis l'IP dans Modifier`);
  await invoke("update_tv", { id: tv.id, name: tv.name, brand: tv.brand, ip: l[0].ip, secret: null, mac: tv.mac ?? null, power: tv.power, irBrand: tv.irBrand ?? null, bt: !!tv.bt });
  await refresh(tv.id);
  return tvs.find((x) => x.id === tv.id) ?? tv;
}

async function envoyer(tv, k) {
  if (peutDecouvrir(tv, k)) tv = await decouvrir(tv);
  // Marche/arrêt : l'infrarouge d'abord (TV en veille = injoignable en Wi-Fi), sauf profil réglé « Wi-Fi seulement ».
  const ordre = k === "power" && tv.power !== "wifi" ? ["ir", "wifi", "bt"] : ["wifi", "ir", "bt"];
  const erreurs = [];
  const noter = (mode, e) => erreurs.push(`${{ wifi: "Wi-Fi", ir: "Infrarouge", bt: "Bluetooth" }[mode]} : ${e?.message ?? e}`);
  for (const mode of ordre) {
    if (!(mode === "wifi" ? okWifi(tv, k) : mode === "ir" ? okIr(tv, k) && (tv.power !== "wifi" || k !== "power") : okBt(tv, k))) continue;
    if (mode === "ir" && !irDispo()) { noter(mode, "ce téléphone n'a pas d'émetteur infrarouge"); continue; }
    try {
      if (mode === "wifi") await invoke("send_key", { id: tv.id, key: k });
      else if (mode === "ir") irSend(irDe(tv), k);
      else await btSend(tv.id, k);
      return;
    } catch (e) { noter(mode, e); }
  }
  throw new Error(erreurs.length ? erreurs.join(" — ") : "Touche non disponible pour ce profil (ajoute un mode : Modifier > Combiner plusieurs modes)");
}

async function press(k) {
  const cs = cibles();
  if (!cs.length) return say("Aucune TV sélectionnée", true);
  navigator.vibrate?.(15);
  const res = await Promise.allSettled(cs.map((tv) => envoyer(tv, k)));
  const ko = res.flatMap((r, i) => (r.status === "rejected" ? [`${cs[i].name} : ${r.reason?.message ?? r.reason}`] : []));
  say(ko.join(" — "), ko.length > 0);
  if (cs.some((t) => !t.paired && NEEDS_PAIR.includes(t.brand.replace(/^ir_/, "")) && t.ip)) refresh(sel.value);
}

// Maintenir volume / chaînes / flèches répète la touche (comme une vraie télécommande) ; clavier = un appui par Entrée.
const REPEAT = new Set(["vol_up", "vol_down", "ch_up", "ch_down", "up", "down", "left", "right"]);
document.querySelectorAll("[data-k]").forEach((b) => {
  const k = b.dataset.k;
  if (!REPEAT.has(k)) return b.addEventListener("click", () => press(k));
  let t = null, busy = false, tenu = false;
  const tir = async () => { if (busy) return; busy = true; try { await press(k); } finally { busy = false; } };
  const stop = () => { clearTimeout(t); clearInterval(t); t = null; };
  b.addEventListener("pointerdown", (e) => { if (e.button) return; tenu = true; tir(); t = setTimeout(() => { t = setInterval(tir, 160); }, 450); });
  ["pointerup", "pointerleave", "pointercancel"].forEach((ev) => b.addEventListener(ev, stop));
  b.addEventListener("click", (e) => { if (e.detail === 0 && !tenu) tir(); tenu = false; }); // détail 0 = activation au clavier
});

// --- Saisie de texte : envoyée au champ de texte actif de la TV (Samsung, LG, Roku) ---
/** Texte : Wi-Fi si la TV a une IP, sinon (ou en repli) Bluetooth ; l'infrarouge ne sait pas taper du texte. */
async function texteVers(tv, t) {
  let err = null;
  if (tv.brand !== "bt" && tv.ip) { try { return await invoke("send_text", { id: tv.id, text: t }); } catch (e) { err = e; } }
  if (tv.brand === "bt" || tv.bt) return btText(tv.id, t);
  throw err ?? new Error("Pas de saisie de texte en infrarouge : ajoute le Wi-Fi (IP) ou le Bluetooth à ce profil");
}
async function envoyerTexte() {
  const t = $("#txt").value; if (!t) return;
  const cs = cibles(); if (!cs.length) return say("Aucune TV sélectionnée", true);
  const res = await Promise.allSettled(cs.map((tv) => texteVers(tv, t)));
  const ko = res.flatMap((r, i) => (r.status === "rejected" ? [`${cs[i].name} : ${r.reason?.message ?? r.reason}`] : []));
  say(ko.length ? ko.join(" — ") : "Texte envoyé (sélectionne d'abord un champ de texte sur la TV)", ko.length > 0);
  if (!ko.length) $("#txt").value = "";
}
$("#txt-go").onclick = envoyerTexte;
$("#txt").addEventListener("keydown", (e) => { if (e.key === "Enter") { e.preventDefault(); envoyerTexte(); } e.stopPropagation(); });

// --- Raccourcis d'applications personnalisables (Plex, Netflix...) : stockés sur l'appareil ---
const APPS_DEFAUT = [{ l: "Plex", a: "plex" }, { l: "Netflix", a: "netflix" }, { l: "YouTube", a: "youtube" }, { l: "Prime", a: "prime video" }, { l: "Disney+", a: "disney" }];
const SUGGESTIONS = [["Apple TV", "apple tv"], ["Spotify", "spotify"], ["Twitch", "twitch"], ["Canal+", "canal"], ["MyCanal", "mycanal"], ["France TV", "france"], ["TF1+", "tf1"], ["6play", "6play"], ["Arte", "arte"],
  ["Crunchyroll", "crunchyroll"], ["Kodi", "kodi"], ["Jellyfin", "jellyfin"], ["Deezer", "deezer"], ["DAZN", "dazn"], ["Paramount+", "paramount"], ["Max", "max"], ["Molotov", "molotov"], ["VLC", "vlc"]];
let apps = (() => { try { const v = JSON.parse(load("apps") ?? "null"); return Array.isArray(v) ? v : APPS_DEFAUT; } catch { return APPS_DEFAUT; } })();
const saveApps = () => { store("apps", JSON.stringify(apps)); drawApps(); };
function drawApps() {
  $("#apps").replaceChildren(...apps.map((x) => {
    const b = document.createElement("button");
    b.className = "b pill" + (x.a === "plex" ? " plex" : ""); b.textContent = x.l; b.title = `Lancer ${x.l} sur la TV`;
    b.dataset.k = `app:${x.a}`; b.onclick = () => press(b.dataset.k);
    return b;
  }));
  $("#apps-list").replaceChildren(...apps.map((x, i) => {
    const r = document.createElement("div"); r.className = "row";
    const t = document.createElement("span"); t.textContent = `${x.l}  →  « ${x.a} »`;
    const d = document.createElement("button"); d.className = "b pill"; d.textContent = "Retirer"; d.onclick = () => { apps.splice(i, 1); saveApps(); };
    r.append(t, d); return r;
  }));
}
function ajouterApp(l, a) {
  l = l.trim(); a = a.trim().toLowerCase();
  if (!l || !a) return say("Indique le nom du bouton et celui de l'appli", true);
  if (apps.some((x) => x.a === a)) return say("Ce raccourci existe déjà", true);
  apps.push({ l: l.slice(0, 14), a: a.slice(0, 40) }); saveApps();
}
$("#apps-sugg").replaceChildren(...SUGGESTIONS.map(([l, a]) => { const c = document.createElement("button"); c.type = "button"; c.className = "chip"; c.textContent = l; c.onclick = () => ajouterApp(l, a); return c; }));
$("#a-add").onclick = () => { ajouterApp($("#a-label").value, $("#a-name").value); $("#a-label").value = ""; $("#a-name").value = ""; };
$("#a-read").onclick = async () => {
  const tv = cur(), st = $("#a-read-st");
  if (!tv) return (st.textContent = "Aucune TV sélectionnée");
  st.textContent = "Lecture des applis de la TV…"; $("#apps-tv").replaceChildren();
  try {
    const l = await invoke("list_apps", { id: tv.id });
    st.textContent = `${l.length} applis : clique pour ajouter un raccourci.`;
    $("#apps-tv").replaceChildren(...l.map((n) => { const c = document.createElement("button"); c.type = "button"; c.className = "chip"; c.textContent = n; c.onclick = () => ajouterApp(n.slice(0, 14), n.toLowerCase()); return c; }));
  } catch (e) { st.textContent = String(e.message ?? e); }
};
$("#a-reset").onclick = () => { apps = APPS_DEFAUT; saveApps(); };
drawApps();
initScenes(press, say, $);
// Diagnostic : ce que l'appli voit (TV joignable ? applis ? émetteur IR ? Bluetooth ?) pour comprendre un bouton qui ne répond pas
$("#diag-go").onclick = async () => {
  const out = $("#diag-out"), tv = cur();
  if (!tv) return (out.textContent = "Aucune TV sélectionnée.");
  out.textContent = "Diagnostic en cours…";
  const l = [`ForgeRemote ${$("#ver").textContent}`, `Plateforme : ${/Android/i.test(navigator.userAgent) ? "Android" : "PC"} ; émetteur infrarouge : ${irDispo() ? "oui" : "non"} ; pont Bluetooth : ${window.ForgeBT ? "oui" : "non"}`];
  l.push(`Modes du profil : Wi-Fi ${tv.brand !== "bt" && tv.ip ? "oui" : "non"} ; infrarouge ${irDe(tv) ? `oui (${irDe(tv)})` : "non"} ; Bluetooth ${tv.brand === "bt" || tv.bt ? "oui" : "non"}`);
  for (const k of ["app:plex", "app:youtube", "vol_up", "power"]) l.push(`Bouton ${k} : Wi-Fi ${okWifi(tv, k) ? "oui" : "non"} / infrarouge ${okIr(tv, k) ? "oui" : "non"} / Bluetooth ${okBt(tv, k) ? "oui" : "non"}`);
  try { l.push(...(await invoke("diagnostic", { id: tv.id }))); } catch (e) { l.push(`Diagnostic réseau : ${e?.message ?? e}`); }
  out.textContent = l.join(String.fromCharCode(10));
};
// Entrées (HDMI...) de la TV LG : liste lue sur la TV, un bouton par entrée
$("#src-read").onclick = async () => {
  const tv = cur(); if (!tv) return;
  try {
    const l = await invoke("list_sources", { id: tv.id });
    $("#srclist").replaceChildren(...l.map((x) => { const b = document.createElement("button"); b.className = "b pill"; b.textContent = x.nom; b.dataset.k = `src:${x.id}`; b.onclick = () => press(b.dataset.k); return b; }));
    say(`${l.length} entrées`);
  } catch (e) { say(String(e.message ?? e), true); }
};
// Android : les boutons de volume du téléphone pilotent la TV (option)
if (window.ForgeVol) {
  window.__vol = (d) => press(d === "up" ? "vol_up" : "vol_down");
  $("#l-volkeys").hidden = false;
  $("#volkeys").checked = load("volkeys") === "1"; window.ForgeVol.set($("#volkeys").checked);
  $("#volkeys").onchange = (e) => { store("volkeys", e.target.checked ? "1" : "0"); window.ForgeVol.set(e.target.checked); };
}

// Clavier (PC) : flèches, Entrée, Échap, chiffres, +/-, M, Page haut/bas, H.
const MAP = { ArrowUp: "up", ArrowDown: "down", ArrowLeft: "left", ArrowRight: "right", Enter: "ok", Escape: "back", Backspace: "back",
  "+": "vol_up", "=": "vol_up", "-": "vol_down", m: "mute", PageUp: "ch_up", PageDown: "ch_down", h: "home" };
addEventListener("keydown", (e) => {
  if (dlg.open || e.target.matches("input,select")) return;
  const k = /^\d$/.test(e.key) ? e.key : MAP[e.key];
  if (!k) return;
  e.preventDefault();
  const b = document.querySelector(`[data-k="${k}"]`);
  b?.classList.add("press"); setTimeout(() => b?.classList.remove("press"), 120);
  press(k);
});

// --- Formulaire : ajout / modification ---
const PIN_MARQUES = ["vizio", "lgnetcast", "firetv", "androidtv"]; // appairage par code affiché sur la TV
const GROUPES = [...brandSel.children];
const modeSel = $("#f-mode");
const modeDe = (b) => (b === "bt" ? "bt" : b.startsWith("ir_") ? "ir" : "wifi");
/** Affiche seulement les marques du mode choisi (Wi-Fi / infrarouge / Bluetooth). */
function filtrerMarques(mode, brand) {
  const g = { wifi: GROUPES.slice(0, 3), bt: [GROUPES[3]], ir: [GROUPES[4]] }[mode];
  brandSel.replaceChildren(...g);
  brandSel.value = brand && [...brandSel.querySelectorAll("option")].some((o) => o.value === brand) ? brand : brandSel.querySelector("option").value;
}
modeSel.onchange = () => { filtrerMarques(modeSel.value); formUi(); };
function formUi() {
  const b = brandSel.value, local = b.startsWith("ir_") || b === "bt";
  $("#l-power").hidden = local; $("#scan").hidden = b === "bt";
  const ir = b.startsWith("ir_");
  $("#l-ip").hidden = b === "bt"; $("#l-mac").hidden = local; $("#detect").hidden = local;
  $("#l-ip-t").textContent = ir ? "IP de la TV (facultatif : pour les applis Plex, Netflix…)" : "Adresse IP de la TV";
  const pinBrand = PIN_MARQUES.includes(b);
  const fbx = b === "freebox";
  $("#l-psk").hidden = b !== "sony" && !pinBrand && !fbx; $("#pin-go").hidden = !pinBrand; $("#l-bt").hidden = b !== "bt";
  const primWifi = !local;
  $("#l-ir-too").hidden = !primWifi; $("#f-ir-too").disabled = !primWifi;
  $("#l-bt-too").hidden = b === "bt"; $("#combo").hidden = b === "bt";
  $("#l-ir-brand").hidden = !(primWifi && $("#f-ir-too").checked);
  if (primWifi && $("#f-ir-too").checked && !$("#f-ir-brand").dataset.touche) $("#f-ir-brand").value = irMarques().some((x) => x.id === `ir_${b}`) ? `ir_${b}` : $("#f-ir-brand").value;
  if ($("#f-bt-too").checked && b !== "bt") { $("#l-bt").hidden = false; remplirBt(); }
  $("#l-psk-t").textContent = fbx ? "Code télécommande réseau de la Freebox" : pinBrand ? "Code affiché sur la TV" : "Clé PSK Sony";
  $("#f-psk").placeholder = fbx ? "Player : Réglages > Système > Informations (8 chiffres)" : pinBrand ? "d'abord « Afficher le code sur la TV », puis saisis-le ici" : "réglée dans les paramètres réseau de la TV";
  $("#f-ip").required = !local;
  if (b === "bt") remplirBt();
}
function remplirBt(choisi) {
  const l = btAppareils();
  $("#f-bt-dev").replaceChildren(...(l.length ? l : [{ name: "Aucun appareil appairé : rends le téléphone visible", address: "" }]).map((d) => new Option(d.name, d.address)));
  if (choisi) $("#f-bt-dev").value = choisi;
}
$("#bt-visible").onclick = () => { btVisible(); setTimeout(() => remplirBt(), 8000); };
brandSel.onchange = formUi;
$("#f-ir-too").onchange = formUi; $("#f-bt-too").onchange = formUi;
$("#f-ir-brand").onchange = () => { $("#f-ir-brand").dataset.touche = "1"; };

function openForm(tv, prefill) {
  editing = tv?.id ?? null;
  $("#form").reset(); $("#detect-res").textContent = ""; delete $("#f-ir-brand").dataset.touche;
  modeSel.value = prefill?.mode ?? (tv ? modeDe(tv.brand) : "wifi");
  filtrerMarques(modeSel.value, tv?.brand);
  $("#f-power").value = tv?.power ?? "auto";
  $("#dlg-title").textContent = tv ? "Modifier la TV" : "Ajouter une TV";
  $("#f-submit").textContent = tv ? "Enregistrer" : "Ajouter";
  if (tv) {
    $("#f-name").value = tv.name; brandSel.value = tv.brand; $("#f-ip").value = tv.ip;
    $("#f-mac").value = tv.mac ?? ""; $("#f-psk").placeholder = "laisser vide = clé inchangée";
    $("#f-ir-too").checked = !!tv.irBrand; if (tv.irBrand) { $("#f-ir-brand").value = tv.irBrand; $("#f-ir-brand").dataset.touche = "1"; }
    $("#f-bt-too").checked = !!tv.bt && tv.brand !== "bt";
  }
  if (prefill) { $("#f-name").value = prefill.name; $("#f-ip").value = prefill.ip ?? ""; $("#f-mac").value = prefill.mac ?? ""; }
  formUi();
  if (tv?.brand === "bt" || tv?.bt) remplirBt(btAdresse(tv.id));
  dlg.showModal();
}
$("#add").onclick = () => openForm();
$("#edit").onclick = () => (cur() ? openForm(cur()) : say("Aucune TV à modifier", true));
/** Nouveau profil de la même TV dans l'autre mode (Wi-Fi <-> infrarouge) : IP et MAC repris. */
$("#dup").onclick = () => {
  const tv = cur();
  if (!tv) return say("Aucune TV à dupliquer", true);
  const mode = modeDe(tv.brand) === "wifi" ? "ir" : "wifi";
  openForm(undefined, { mode, name: `${tv.name} (${mode === "ir" ? "IR" : "Wi-Fi"})`, ip: tv.ip, mac: tv.mac });
  if (mode === "ir") { const m = irMarques().find((x) => x.id === `ir_${tv.brand}`); if (m) brandSel.value = m.id; }
  else brandSel.value = "auto";
  formUi();
};
$("#pin-go").onclick = async () => {
  const out = $("#detect-res");
  try { await invoke("code_afficher", { brand: brandSel.value, ip: $("#f-ip").value }); out.textContent = "Code affiché sur la TV : saisis-le dans le champ ci-dessous, puis Ajouter/Enregistrer."; }
  catch (e) { out.textContent = String(e.message ?? e); }
};
$("#f-cancel").onclick = () => dlg.close();

$("#detect").onclick = async () => {
  const out = $("#detect-res"); out.textContent = "Détection en cours…";
  try {
    const r = await invoke("detect_tv", { ip: $("#f-ip").value });
    if (!r) return (out.textContent = `Aucune TV reconnue à cette adresse. ${AIDE_RESEAU}`);
    brandSel.value = r.brand; formUi();
    if (!$("#f-name").value) $("#f-name").value = r.name.slice(0, 40);
    out.textContent = `Reconnue : ${r.name} (${r.brand})`;
  } catch (e) { out.textContent = String(e); }
};

// --- Recherche réseau : fenêtre au premier plan avec l'avancement et le résultat ---
const scanDlg = $("#scan-dlg");
async function lancerScan() {
  const st = $("#scan-st"), list = $("#scan-list");
  list.replaceChildren(); $("#scan-again").disabled = true;
  st.textContent = "Recherche en cours sur ton réseau local (environ 8 secondes)…";
  try {
    const r = await invoke("scan_tvs");
    list.replaceChildren(...r.map((d) => {
      const c = document.createElement("button");
      c.type = "button"; c.className = "chip"; c.textContent = `${d.name} · ${d.ip} (${d.brand})`;
      c.onclick = () => {
        if (modeSel.value !== "ir") { modeSel.value = "wifi"; filtrerMarques("wifi", d.brand); }
        $("#f-ip").value = d.ip;
        if (!$("#f-name").value) $("#f-name").value = String(d.name).slice(0, 40);
        formUi(); scanDlg.close();
      };
      return c;
    }));
    st.textContent = r.length ? `${r.length} TV trouvée(s) : clique sur la tienne.`
      : `Aucune TV trouvée. Une TV éteinte ou en veille ne répond pas au réseau : allume-la puis relance. ${AIDE_RESEAU} Sinon saisis l'IP et clique « Détecter ».`;
  } catch (e) { st.textContent = String(e.message ?? e); }
  $("#scan-again").disabled = false;
}
$("#scan").onclick = () => { scanDlg.showModal(); lancerScan(); };
$("#scan-again").onclick = lancerScan;
$("#scan-close").onclick = () => scanDlg.close();

$("#form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const f = { name: $("#f-name").value, ip: $("#f-ip").value, secret: $("#f-psk").value || null, mac: $("#f-mac").value || null, power: $("#f-power").value,
    irBrand: $("#f-ir-too").checked && !$("#f-ir-too").disabled ? $("#f-ir-brand").value : null, bt: $("#f-bt-too").checked && brandSel.value !== "bt" };
  let brand = brandSel.value;
  try {
    if (brand === "auto") { // mode auto : on identifie la marque d'après l'IP
      const r = await invoke("detect_tv", { ip: f.ip });
      if (!r) throw new Error(`Aucune TV reconnue à cette adresse. ${AIDE_RESEAU}`);
      brand = r.brand;
    }
    if (PIN_MARQUES.includes(brand) && $("#f-psk").value.trim()) f.secret = await invoke("code_finir", { brand, ip: f.ip, pin: $("#f-psk").value.trim() });
    if ((brand === "bt" || f.bt) && !$("#f-bt-dev").value) throw new Error("Choisis la TV Bluetooth dans la liste (appaire d'abord le téléphone avec elle)");
    const t = await invoke(editing ? "update_tv" : "add_tv", { ...(editing ? { id: editing } : {}), ...f, brand });
    if (brand === "bt" || f.bt) { try { localStorage.setItem(`bt:${t.id}`, $("#f-bt-dev").value); } catch {} }
    dlg.close();
    if (!f.mac && brand !== "bt" && !brand.startsWith("ir_")) invoke("learn_mac", { id: t.id }).then(() => refresh(t.id)).catch(() => {});
    await refresh(t.id);
    say(`${t.name} : ${editing ? "modifiée" : "ajoutée"} (${t.brand})`);
  } catch (err) { $("#detect-res").textContent = String(err.message ?? err); }
});

$("#del").onclick = async () => {
  const tv = cur();
  if (tv && confirm(`Retirer « ${tv.name} » ? Elle ne sera plus jamais contactée.`)) { await invoke("remove_tv", { id: tv.id }); try { localStorage.removeItem(`bt:${tv.id}`); } catch {} refresh(); }
};
sel.onchange = () => refresh(sel.value);
$("#grp").onclick = () => {
  grpOn = !grpOn; $("#grp").setAttribute("aria-pressed", String(grpOn)); $("#group").hidden = !grpOn; majTouches();
  say(grpOn ? "Mode groupe : les touches vont à toutes les TV cochées" : "");
};

// --- Mise à jour ---
// Un clic sur « Installer » : téléchargement + SHA-256 + installation + relance, sans passer par GitHub.
// Windows installé : installeur silencieux ; portable : exe remplacé ; Android : APK vérifié puis écran système « Installer ».
let majEnCours = null;
function proposer(u) {
  majEnCours = u;
  $("#updtxt").textContent = `ForgeRemote ${u.version} est disponible (installée : ${$("#ver").textContent.replace(/^v/, "")})`;
  $("#updbar").hidden = false;
  $("#updgo").disabled = false;
}
$("#updno").onclick = () => { $("#updbar").hidden = true; };
$("#updgo").onclick = async () => {
  const u = majEnCours; if (!u) return;
  $("#updgo").disabled = true; say("Téléchargement et vérification de l'intégrité…");
  try {
    if (window.ForgeUpd) await installerAndroid(u);
    else if (u.kind === "setup" || u.kind === "portable" || u.kind === "appimage") { await invoke("install_update", { url: u.url, name: u.name, sums: u.sums }); say("Installation…"); }
    else { await invoke("open_release", { url: u.page }); say("Page de la release ouverte : télécharge le paquet pour ton système."); }
  } catch (e) { say(String(e.message ?? e), true); $("#updgo").disabled = false; }
};
async function installerAndroid(u) {
  if (!u.apk || !u.sums) throw new Error(`APK absent de la release : ${u.page}`);
  const somme = await invoke("somme_update", { sums: u.sums, name: u.apk.name }); // signature Ed25519 vérifiée côté Rust
  window.ForgeUpd.install(u.apk.url, u.apk.name, somme);
  let confirme = false;
  for (let n = 0; n < 400; n++) { // ~4,5 min : assez pour lire l'avertissement Play Protect
    await new Promise((r) => setTimeout(r, 700));
    const st = window.ForgeUpd.status();
    if (st === "perm") say("Autorise « Installer des applis inconnues » pour ForgeRemote, puis reviens : l'installation reprendra seule.");
    else if (st === "ok" && !confirme) { confirme = true; say("Confirme l'installation sur l'écran Android (si Play Protect avertit : « Installer quand même »)."); }
    else if (st === "fini") return say("Mise à jour installée.");
    else if (st.startsWith("err:")) throw new Error(st.slice(4));
  }
  throw new Error("pas de réponse de l'installeur Android : ouvre la page de la release et installe l'APK à la main");
}
async function checkUpdate(manuel) {
  try {
    const u = await invoke("check_update");
    if (!u) return manuel && say("ForgeRemote est à jour");
    proposer(u);
    if (manuel) say(`Version ${u.version} disponible : clique sur Installer`);
  } catch (e) { manuel && say(String(e), true); }
}
$("#upd").onclick = () => checkUpdate(true);
$("#autoupd").checked = load("noupd") !== "1";
$("#autoupd").onchange = (e) => store("noupd", e.target.checked ? undefined : "1");
if ($("#autoupd").checked) setTimeout(() => checkUpdate(false), 3000);
setInterval(() => $("#autoupd").checked && checkUpdate(false), 6 * 3600 * 1000);

initLook(press);
refresh(load("tv"));
