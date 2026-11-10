import { irSend } from "./ir.js";

const invoke = window.__TAURI__.core.invoke;
const $ = (s) => document.querySelector(s);
const sel = $("#tv"), msg = $("#msg"), dlg = $("#dlg");
let tvs = [];

const say = (t, err) => { msg.textContent = t; msg.className = err ? "err" : ""; };
const cur = () => tvs.find((t) => t.id === sel.value);
const store = (k, v) => { try { v === undefined ? localStorage.removeItem(k) : localStorage.setItem(k, v); } catch {} };
const load = (k) => { try { return localStorage.getItem(k); } catch { return null; } };
const NEEDS_PAIR = ["samsung", "lg"];

async function refresh(keep) {
  tvs = await invoke("list_tvs");
  sel.replaceChildren(...tvs.map((t) => new Option(`${t.name} (${t.brand})`, t.id)));
  if (keep) sel.value = keep;
  if (!tvs.length) say("Ajoute ta première TV avec le bouton +");
  else if (!cur().paired && NEEDS_PAIR.includes(cur().brand)) say("TV non appairée : appuie sur une touche et accepte sur la TV");
  else say("");
  store("tv", sel.value);
}

async function press(k) {
  const tv = cur();
  if (!tv) return say("Aucune TV sélectionnée", true);
  navigator.vibrate?.(15);
  try {
    if (tv.ir) irSend(tv.brand, k);
    else await invoke("send_key", { id: tv.id, key: k });
    say("");
    if (!tv.paired && NEEDS_PAIR.includes(tv.brand)) refresh(tv.id);
  } catch (e) { say(String(e.message ?? e), true); }
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

// --- Ajout d'une TV (liste blanche) ---
const brandSel = $("#f-brand");
function formUi() {
  const b = brandSel.value, ir = b.startsWith("ir_");
  $("#l-ip").hidden = ir; $("#l-mac").hidden = ir; $("#l-psk").hidden = b !== "sony";
  $("#f-ip").required = !ir;
}
brandSel.onchange = formUi;
$("#add").onclick = () => { formUi(); dlg.showModal(); };
$("#f-cancel").onclick = () => dlg.close();

$("#scan").onclick = async () => {
  const box = $("#found"); box.replaceChildren(); $("#scan").disabled = true; say("Recherche en cours (3 s)…");
  try {
    const r = await invoke("scan_tvs");
    box.replaceChildren(...r.map((d) => {
      const c = document.createElement("button");
      c.type = "button"; c.className = "chip"; c.textContent = `${d.brand} ${d.ip}`;
      c.onclick = () => { brandSel.value = d.brand; $("#f-ip").value = d.ip; formUi(); };
      return c;
    }));
    say(r.length ? "" : "Aucune TV trouvée : saisis l'adresse IP à la main");
  } catch (e) { say(String(e), true); }
  $("#scan").disabled = false;
};

$("#form").addEventListener("submit", async () => {
  try {
    const t = await invoke("add_tv", { name: $("#f-name").value, brand: brandSel.value, ip: $("#f-ip").value,
      secret: $("#f-psk").value || null, mac: $("#f-mac").value || null });
    $("#form").reset(); $("#found").replaceChildren();
    await refresh(t.id);
  } catch (e) { say(String(e), true); }
});
$("#del").onclick = async () => {
  const tv = cur();
  if (tv && confirm(`Retirer « ${tv.name} » ? Elle ne sera plus jamais contactée.`)) { await invoke("remove_tv", { id: tv.id }); refresh(); }
};
sel.onchange = () => refresh(sel.value);

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

refresh(load("tv"));
