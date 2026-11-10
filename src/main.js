import { irSend } from "./ir.js";
import { initLook } from "./look.js";

const invoke = window.__TAURI__.core.invoke;
const $ = (s) => document.querySelector(s);
const sel = $("#tv"), msg = $("#msg"), dlg = $("#dlg"), brandSel = $("#f-brand");
let tvs = [], editing = null, grpOn = false;

const say = (t, err) => { msg.textContent = t; msg.className = err ? "err" : ""; };
const cur = () => tvs.find((t) => t.id === sel.value);
const store = (k, v) => { try { v === undefined ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch {} };
const load = (k) => { try { return localStorage.getItem(k); } catch { return null; } };
const NEEDS_PAIR = ["samsung", "lg"];
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
    c.onchange = () => store("grp", JSON.stringify([...document.querySelectorAll("#group input")].flatMap((x, i) => (x.checked ? [tvs[i].id] : []))));
    l.append(c, ` ${t.name}`);
    return l;
  }));
}

async function refresh(keep) {
  tvs = await invoke("list_tvs");
  sel.replaceChildren(...tvs.map((t) => new Option(`${t.name} (${t.brand})`, t.id)));
  if (keep) sel.value = keep;
  buildGroup();
  if (!tvs.length) say("Ajoute ta première TV avec le bouton Ajouter");
  else if (!cur().paired && NEEDS_PAIR.includes(cur().brand)) say("TV non appairée : appuie sur une touche et accepte sur la TV");
  else say("");
  store("tv", sel.value);
}

async function envoyer(tv, k) {
  if (tv.ir) irSend(tv.brand, k);
  else await invoke("send_key", { id: tv.id, key: k });
}

async function press(k) {
  const cs = cibles();
  if (!cs.length) return say("Aucune TV sélectionnée", true);
  navigator.vibrate?.(15);
  const res = await Promise.allSettled(cs.map((tv) => envoyer(tv, k)));
  const ko = res.flatMap((r, i) => (r.status === "rejected" ? [`${cs[i].name} : ${r.reason?.message ?? r.reason}`] : []));
  say(ko.join(" — "), ko.length > 0);
  if (cs.some((t) => !t.paired && NEEDS_PAIR.includes(t.brand))) refresh(sel.value);
}

document.querySelectorAll("[data-k]").forEach((b) => b.addEventListener("click", () => press(b.dataset.k)));

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
function formUi() {
  const b = brandSel.value, ir = b.startsWith("ir_");
  $("#l-ip").hidden = ir; $("#l-mac").hidden = ir; $("#detect").hidden = ir;
  $("#l-psk").hidden = b !== "sony";
  $("#f-ip").required = !ir;
}
brandSel.onchange = formUi;

function openForm(tv) {
  editing = tv?.id ?? null;
  $("#form").reset(); $("#found").replaceChildren(); $("#detect-res").textContent = "";
  $("#dlg-title").textContent = tv ? "Modifier la TV" : "Ajouter une TV";
  $("#f-submit").textContent = tv ? "Enregistrer" : "Ajouter";
  if (tv) {
    $("#f-name").value = tv.name; brandSel.value = tv.brand; $("#f-ip").value = tv.ip;
    $("#f-mac").value = tv.mac ?? ""; $("#f-psk").placeholder = "laisser vide = clé inchangée";
  }
  formUi(); dlg.showModal();
}
$("#add").onclick = () => openForm();
$("#edit").onclick = () => (cur() ? openForm(cur()) : say("Aucune TV à modifier", true));
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

$("#scan").onclick = async () => {
  const box = $("#found"); box.replaceChildren(); $("#scan").disabled = true; say("Recherche sur le réseau (jusqu'à 8 s)…");
  try {
    const r = await invoke("scan_tvs");
    box.replaceChildren(...r.map((d) => {
      const c = document.createElement("button");
      c.type = "button"; c.className = "chip"; c.textContent = `${d.name} · ${d.ip}`;
      c.onclick = () => { brandSel.value = d.brand; $("#f-ip").value = d.ip; if (!$("#f-name").value) $("#f-name").value = String(d.name).slice(0, 40); formUi(); };
      return c;
    }));
    say(r.length ? "" : `Aucune TV trouvée. ${AIDE_RESEAU} Sinon saisis l'IP et clique « Détecter ».`);
  } catch (e) { say(String(e), true); }
  $("#scan").disabled = false;
};

$("#form").addEventListener("submit", async (e) => {
  e.preventDefault();
  const f = { name: $("#f-name").value, ip: $("#f-ip").value, secret: $("#f-psk").value || null, mac: $("#f-mac").value || null };
  let brand = brandSel.value;
  try {
    if (brand === "auto") { // mode auto : on identifie la marque d'après l'IP
      const r = await invoke("detect_tv", { ip: f.ip });
      if (!r) throw new Error(`Aucune TV reconnue à cette adresse. ${AIDE_RESEAU}`);
      brand = r.brand;
    }
    const t = await invoke(editing ? "update_tv" : "add_tv", { ...(editing ? { id: editing } : {}), ...f, brand });
    dlg.close();
    await refresh(t.id);
    say(`${t.name} : ${editing ? "modifiée" : "ajoutée"} (${t.brand})`);
  } catch (err) { $("#detect-res").textContent = String(err.message ?? err); }
});

$("#del").onclick = async () => {
  const tv = cur();
  if (tv && confirm(`Retirer « ${tv.name} » ? Elle ne sera plus jamais contactée.`)) { await invoke("remove_tv", { id: tv.id }); refresh(); }
};
sel.onchange = () => refresh(sel.value);
$("#grp").onclick = () => {
  grpOn = !grpOn; $("#grp").setAttribute("aria-pressed", String(grpOn)); $("#group").hidden = !grpOn;
  say(grpOn ? "Mode groupe : les touches vont à toutes les TV cochées" : "");
};

// --- Mise à jour ---
async function checkUpdate(manuel) {
  try {
    const u = await invoke("check_update");
    if (!u) return manuel && say("ForgeRemote est à jour");
    if (u.url && u.sums && confirm(`Version ${u.version} disponible. Installer maintenant ?`)) {
      say("Téléchargement et vérification de l'intégrité…");
      await invoke("install_update", { url: u.url, name: u.name, sums: u.sums });
    } else say(`Version ${u.version} disponible : ${u.page}`);
  } catch (e) { manuel && say(String(e), true); }
}
$("#upd").onclick = () => checkUpdate(true);
$("#autoupd").checked = load("noupd") !== "1";
$("#autoupd").onchange = (e) => store("noupd", e.target.checked ? undefined : "1");
if ($("#autoupd").checked) setTimeout(() => checkUpdate(false), 3000);
setInterval(() => $("#autoupd").checked && checkUpdate(false), 6 * 3600 * 1000);

initLook(press);
refresh(load("tv"));
