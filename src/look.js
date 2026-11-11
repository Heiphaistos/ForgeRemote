// Apparence : thèmes (data-theme sur <html>), dispositions (data-layout sur .remote), relief 3D (data-fx) et inclinaison.
// « Studio » (relief 3D) est le défaut ; « Classique » (rendu d'origine) reste disponible dans les Paramètres.
const THEMES = [
  { id: "studio", name: "Studio", sw: ["#3a3f4d", "#4f8cff"] },
  { id: "classique", name: "Classique", sw: ["#1a1c23", "#4f8cff"] },
  { id: "aluminium", name: "Aluminium", sw: ["#cfd3da", "#2f6bff"] },
  { id: "carbone", name: "Carbone", sw: ["#15171b", "#ff3b3b"] },
  { id: "or", name: "Champagne", sw: ["#cdb27a", "#6b4a12"] },
  { id: "rubis", name: "Rubis", sw: ["#5a0f1e", "#ff4d6d"] },
  { id: "saphir", name: "Saphir", sw: ["#0b2a66", "#5fa8ff"] },
  { id: "aurore", name: "Aurore", sw: ["#1fd1a5", "#8a5cff"] },
  { id: "cyber", name: "Cyber", sw: ["#14141a", "#f5e61b"] },
  { id: "lave", name: "Lave", sw: ["#2a0d08", "#ff6a1a"] },
  { id: "glace", name: "Glace", sw: ["#cfeaf7", "#1aa7e0"] },
  { id: "couchant", name: "Couchant", sw: ["#ff7a45", "#7b3fe4"] },
  { id: "menthe", name: "Menthe", sw: ["#c8f0dc", "#12a36a"] },
  { id: "lavande", name: "Lavande", sw: ["#d9ccf5", "#7a4be0"] },
  { id: "minuit", name: "Minuit", sw: ["#0a0d2a", "#9aa8ff"] },
  { id: "verre", name: "Verre", sw: ["#6a5cff", "#00c8ff"] },
  { id: "neon", name: "Néon", sw: ["#0d0b18", "#00f0ff"] },
  { id: "clair", name: "Clair", sw: ["#e4e8f0", "#4f6bff"] },
  { id: "amoled", name: "AMOLED", sw: ["#000000", "#ff7a18"] },
  { id: "retro", name: "Rétro", sw: ["#e8d6ae", "#e0782a"] },
  { id: "ocean", name: "Océan", sw: ["#082540", "#2bd4ff"] },
  { id: "foret", name: "Forêt", sw: ["#14301f", "#7bd88f"] },
  { id: "plex", name: "Plex", sw: ["#1f2326", "#e5a00d"] },
  { id: "sakura", name: "Sakura", sw: ["#f7dfe7", "#ff7aa8"] },
  { id: "emeraude", name: "Émeraude", sw: ["#0b4a35", "#3dffb0"] },
  { id: "amethyste", name: "Améthyste", sw: ["#3b1670", "#c58bff"] },
  { id: "corail", name: "Corail", sw: ["#6b2a24", "#ff8a75"] },
  { id: "turquoise", name: "Turquoise", sw: ["#0b4f55", "#2ef0e0"] },
  { id: "bordeaux", name: "Bordeaux", sw: ["#3c0f20", "#ff7f9f"] },
  { id: "graphite", name: "Graphite", sw: ["#2b2e33", "#9ba3b0"] },
  { id: "bronze", name: "Bronze", sw: ["#5a3b1c", "#ffb35c"] },
  { id: "arctique", name: "Arctique", sw: ["#dbe9f3", "#2b7fd0"] },
  { id: "citron", name: "Citron", sw: ["#e9e27a", "#8a7d00"] },
  { id: "bubblegum", name: "Bubblegum", sw: ["#ffb3de", "#e0289f"] },
  { id: "matrix", name: "Matrix", sw: ["#021a07", "#2bff5a"] },
  { id: "sunset2", name: "Désert", sw: ["#a35a2c", "#6e2a0a"] },
  { id: "galaxie", name: "Galaxie", sw: ["#1b1040", "#ff6bd6"] },
  { id: "steel", name: "Acier bleu", sw: ["#27394d", "#6fb7ff"] },
];
// size = taille de fenêtre conseillée sur PC [largeur, hauteur]
const LAYOUTS = [
  { id: "complete", name: "Complète", note: "Tous les boutons", size: [400, 960] },
  { id: "compact", name: "Compacte", note: "L'essentiel", size: [400, 700] },
  { id: "wide", name: "Horizontale", note: "2 colonnes (PC)", size: [860, 700] },
  { id: "touch", name: "Tactile", note: "Glisser = flèches", size: [400, 760] },
  { id: "media", name: "Média", note: "Lecture en grand", size: [400, 840] },
  { id: "onehand", name: "Une main", note: "Pouce en bas", size: [380, 760] },
  { id: "left", name: "Gaucher", note: "Inversée", size: [400, 960] },
  { id: "gamer", name: "Manette", note: "Grand D-pad", size: [400, 700] },
  { id: "minimal", name: "Minimale", note: "Quatre gestes", size: [340, 560] },
  { id: "digits", name: "Chiffres", note: "Gros pavé 0-9", size: [400, 860] },
  { id: "cinema", name: "Cinéma", note: "Lecture + volume", size: [400, 760] },
  { id: "stream", name: "Streaming", note: "Applis d'abord", size: [400, 760] },
  { id: "xl", name: "Grands boutons", note: "Lisible de loin", size: [440, 1020] },
  { id: "mini", name: "Mini", note: "Petite fenêtre", size: [290, 470] },
  { id: "tablet3", name: "Tablette 3 col.", note: "Trois colonnes", size: [1000, 640] },
  { id: "tvclassic", name: "TV classique", note: "Comme l'original", size: [400, 960] },
  { id: "dense", name: "Dense", note: "Tout serré", size: [360, 820] },
  { id: "bigvol", name: "Volume géant", note: "Gros +/- et CH", size: [400, 820] },
  { id: "appsfirst", name: "Applis en tête", note: "Raccourcis en haut", size: [400, 960] },
  { id: "zen", name: "Zen", note: "OK, volume, power", size: [340, 560] },
];

const load = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
const save = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
const ok = (list, id, d) => (list.some((x) => x.id === id) ? id : d);
const tactile = () => matchMedia("(hover: none), (pointer: coarse)").matches || matchMedia("(prefers-reduced-motion: reduce)").matches;

/** Sur PC, adapte la taille de la fenêtre à la disposition ; ignoré sur Android. */
async function resize([w, h]) {
  try {
    const win = window.__TAURI__.window;
    if (/Android/i.test(navigator.userAgent)) return;
    await win.getCurrentWindow().setSize(new win.LogicalSize(w, h));
  } catch {}
}

function apply(initial) {
  const t = ok(THEMES, load("theme", "studio"), "studio");
  const l = ok(LAYOUTS, load("layout", "complete"), "complete");
  document.documentElement.dataset.theme = t;
  document.documentElement.dataset.fx = load("fx3d", "1") === "1" ? "3d" : "flat";
  document.querySelector(".remote").dataset.layout = l;
  if (!initial) resize(LAYOUTS.find((x) => x.id === l).size);
}

function chips(box, list, key, defaut, bg) {
  const cur = ok(list, load(key, defaut), defaut);
  box.replaceChildren(...list.map((o) => {
    const b = document.createElement("button");
    b.type = "button"; b.className = "opt"; b.setAttribute("aria-pressed", String(o.id === cur)); b.title = o.note ?? o.name;
    const sw = document.createElement("span"); sw.className = "sw";
    if (bg) sw.style.background = `linear-gradient(135deg, ${o.sw[0]} 50%, ${o.sw[1]} 50%)`; else sw.hidden = true;
    const txt = document.createElement("span"); txt.textContent = o.name;
    if (o.note) { const sm = document.createElement("small"); sm.textContent = o.note; txt.append(sm); }
    b.append(sw, txt);
    b.onclick = () => { save(key, o.id); apply(false); chips(box, list, key, defaut, bg); };
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

/** Inclinaison 3D : la télécommande suit le pointeur (±5°) avec un reflet qui glisse dessus. */
function initTilt() {
  const r = document.querySelector(".remote");
  const actif = () => document.documentElement.dataset.fx === "3d" && load("tilt", tactile() ? "0" : "1") === "1" && !tactile();
  let raf = 0;
  addEventListener("pointermove", (e) => {
    if (!actif() || document.querySelector("dialog[open]")) return;
    cancelAnimationFrame(raf);
    raf = requestAnimationFrame(() => {
      const b = r.getBoundingClientRect();
      const px = Math.min(1, Math.max(0, (e.clientX - b.left) / b.width)), py = Math.min(1, Math.max(0, (e.clientY - b.top) / b.height));
      r.style.setProperty("--ry", `${((px - 0.5) * 10).toFixed(2)}deg`);
      r.style.setProperty("--rx", `${((0.5 - py) * 8).toFixed(2)}deg`);
      r.style.setProperty("--gx", `${(px * 100).toFixed(0)}%`); r.style.setProperty("--gy", `${(py * 100).toFixed(0)}%`);
    });
  });
  document.addEventListener("pointerleave", () => { r.style.setProperty("--rx", "0deg"); r.style.setProperty("--ry", "0deg"); });
}

export function initLook(press) {
  apply(true);
  const dlg = document.querySelector("#set-dlg");
  const ouvrir = () => {
    chips(document.querySelector("#themes"), THEMES, "theme", "studio", true);
    chips(document.querySelector("#layouts"), LAYOUTS, "layout", "complete", false);
    document.querySelector("#fx3d").checked = load("fx3d", "1") === "1";
    document.querySelector("#tilt").checked = load("tilt", tactile() ? "0" : "1") === "1";
    document.querySelector("#maj-ver").textContent = `Version installée : ${document.querySelector("#ver").textContent}`;
    dlg.showModal();
  };
  document.querySelector("#settings").onclick = ouvrir;
  document.querySelector("#set-close").onclick = () => dlg.close();
  document.querySelector("#fx3d").onchange = (e) => { save("fx3d", e.target.checked ? "1" : "0"); apply(true); };
  document.querySelector("#tilt").onchange = (e) => save("tilt", e.target.checked ? "1" : "0");
  document.querySelectorAll("[data-tab]").forEach((t) => (t.onclick = () => {
    document.querySelectorAll("[data-tab]").forEach((x) => x.setAttribute("aria-pressed", String(x === t)));
    document.querySelectorAll("[data-pane]").forEach((p) => (p.hidden = p.dataset.pane !== t.dataset.tab));
  }));
  document.querySelector("#num").onclick = () => document.querySelector(".pad").classList.toggle("open");
  initTouch(press);
  initTilt();
  const l = load("layout", "complete");
  if (l !== "complete") resize(LAYOUTS.find((x) => x.id === l)?.size ?? [400, 960]);
}
