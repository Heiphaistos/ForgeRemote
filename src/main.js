const invoke = window.__TAURI__.core.invoke;
const $ = (s) => document.querySelector(s);
const sel = $("#tv"), msg = $("#msg"), dlg = $("#dlg");
let tvs = [];

const say = (t, err) => { msg.textContent = t; msg.className = err ? "err" : ""; };
const cur = () => tvs.find((t) => t.id === sel.value);

async function refresh(keep) {
  tvs = await invoke("list_tvs");
  sel.replaceChildren(...tvs.map((t) => new Option(`${t.name} (${t.brand})`, t.id)));
  if (keep) sel.value = keep;
  if (!tvs.length) say("Ajoute ta première TV avec le bouton +");
  else if (!cur().paired && cur().brand !== "panasonic") say("TV non appairée : appuie sur une touche et accepte sur la TV");
  else say("");
  try { localStorage.setItem("tv", sel.value); } catch {}
}

async function press(k) {
  const tv = cur();
  if (!tv) return say("Aucune TV sélectionnée", true);
  navigator.vibrate?.(15);
  try {
    await invoke("send_key", { id: tv.id, key: k });
    say("");
    if (!tv.paired) refresh(tv.id);
  } catch (e) { say(String(e), true); }
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

$("#add").onclick = () => dlg.showModal();
$("#f-cancel").onclick = () => dlg.close();
$("#form").addEventListener("submit", async () => {
  try {
    const t = await invoke("add_tv", { name: $("#f-name").value, brand: $("#f-brand").value, ip: $("#f-ip").value });
    $("#form").reset();
    await refresh(t.id);
  } catch (e) { say(String(e), true); }
});
$("#del").onclick = async () => {
  const tv = cur();
  if (tv && confirm(`Retirer « ${tv.name} » ? Elle ne sera plus jamais contactée.`)) { await invoke("remove_tv", { id: tv.id }); refresh(); }
};
sel.onchange = () => refresh(sel.value);

let last = null;
try { last = localStorage.getItem("tv"); } catch {}
refresh(last);
