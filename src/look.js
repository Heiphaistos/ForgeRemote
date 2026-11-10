// Apparence : esthétiques (data-theme sur <html>) et dispositions (data-layout sur .remote).
// Le rendu d'origine (classique + complète) reste le défaut tant que l'utilisateur ne choisit rien.
const THEMES = [
  { id: "classique", name: "Classique", note: "Sombre, d'origine", sw: ["#1a1c23", "#4f8cff"] },
  { id: "verre", name: "Verre", note: "Translucide, dégradé", sw: ["#6a5cff", "#00c8ff"] },
  { id: "neon", name: "Néon", note: "Contours lumineux", sw: ["#0d0b18", "#00f0ff"] },
  { id: "clair", name: "Clair", note: "Relief doux", sw: ["#e4e8f0", "#4f6bff"] },
  { id: "amoled", name: "AMOLED", note: "Noir pur, orange", sw: ["#000000", "#ff7a18"] },
  { id: "retro", name: "Rétro", note: "Crème et brun", sw: ["#e8d6ae", "#e0782a"] },
];
const LAYOUTS = [
  { id: "complete", name: "Complète", note: "Tous les boutons", size: [400, 900] },
  { id: "compact", name: "Compacte", note: "L'essentiel, 123 replié", size: [400, 640] },
  { id: "wide", name: "Horizontale", note: "Deux colonnes (PC)", size: [860, 640] },
  { id: "touch", name: "Tactile", note: "Glisser = flèches", size: [400, 700] },
  { id: "media", name: "Média", note: "Lecture en grand", size: [400, 780] },
];

const load = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
const save = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
const ok = (list, id, d) => (list.some((x) => x.id === id) ? id : d);

/** Sur PC, adapte la taille de la fenêtre à la disposition ; ignoré sur Android. */
async function resize([w, h]) {
  try {
    const win = window.__TAURI__.window;
    if (/Android/i.test(navigator.userAgent)) return;
    await win.getCurrentWindow().setSize(new win.LogicalSize(w, h));
  } catch {}
}

function apply(initial) {
  const t = ok(THEMES, load("theme", "classique"), "classique");
  const l = ok(LAYOUTS, load("layout", "complete"), "complete");
  document.documentElement.dataset.theme = t;
  document.querySelector(".remote").dataset.layout = l;
  if (!initial) resize(LAYOUTS.find((x) => x.id === l).size);
}

function chips(box, list, key, bg) {
  const cur = load(key, list[0].id);
  box.replaceChildren(...list.map((o) => {
    const b = document.createElement("button");
    b.type = "button"; b.className = "opt"; b.setAttribute("aria-pressed", String(o.id === cur));
    const sw = document.createElement("span"); sw.className = "sw";
    if (bg) sw.style.background = `linear-gradient(135deg, ${o.sw[0]} 50%, ${o.sw[1]} 50%)`;
    else sw.hidden = true;
    const txt = document.createElement("span"); txt.textContent = o.name;
    const sm = document.createElement("small"); sm.textContent = o.note; txt.append(sm);
    b.append(sw, txt);
    b.onclick = () => { save(key, o.id); apply(false); chips(box, list, key, bg); };
    return b;
  }));
}

/** Pavé tactile : glisser = flèche, toucher = OK (avec retour visuel). */
function initTouch(press) {
  const pad = document.querySelector("#touch");
  let from = null;
  pad.addEventListener("pointerdown", (e) => { from = [e.clientX, e.clientY]; pad.setPointerCapture(e.pointerId); });
  pad.addEventListener("pointerup", (e) => {
    if (!from) return;
    const dx = e.clientX - from[0], dy = e.clientY - from[1]; from = null;
    const k = Math.hypot(dx, dy) < 14 ? "ok" : Math.abs(dx) > Math.abs(dy) ? (dx > 0 ? "right" : "left") : (dy > 0 ? "down" : "up");
    pad.classList.add("hit"); setTimeout(() => pad.classList.remove("hit"), 140);
    press(k);
  });
  pad.addEventListener("pointercancel", () => { from = null; });
}

export function initLook(press) {
  apply(true);
  const dlg = document.querySelector("#look-dlg");
  document.querySelector("#look").onclick = () => {
    chips(document.querySelector("#themes"), THEMES, "theme", true);
    chips(document.querySelector("#layouts"), LAYOUTS, "layout", false);
    dlg.showModal();
  };
  document.querySelector("#look-close").onclick = () => dlg.close();
  document.querySelector("#num").onclick = () => document.querySelector(".pad").classList.toggle("open");
  initTouch(press);
  if (load("layout", "complete") !== "complete") resize(LAYOUTS.find((x) => x.id === load("layout", "complete"))?.size ?? [400, 900]);
}
