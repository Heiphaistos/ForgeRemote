// Scènes (suites de touches, ex. « power 5s hdmi1 ») et saisie directe d'un numéro de chaîne.
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const load = (k, d) => { try { return localStorage.getItem(k) ?? d; } catch { return d; } };
const save = (k, v) => { try { localStorage.setItem(k, v); } catch {} };
const PRESETS = [["Allumer + HDMI 1", "power 5s hdmi1"], ["Éteindre", "power"], ["Silence 5 s", "mute 5s mute"], ["Chaîne 1", "1 ok"]];

/** « power 5s hdmi1 » -> [{k:"power"},{wait:5000},{k:"hdmi1"}] ; max 30 étapes, attente max 30 s. */
export function parse(txt) {
  return String(txt).trim().split(/[\s,;]+/).filter(Boolean).slice(0, 30).map((t) => {
    const m = /^(\d+(?:\.\d)?)s$/.exec(t);
    return m ? { wait: Math.min(30, Number(m[1])) * 1000 } : { k: t.slice(0, 44) };
  });
}

export function initScenes(press, say, $) {
  let list = (() => { try { const v = JSON.parse(load("scenes", "[]")); return Array.isArray(v) ? v : []; } catch { return []; } })();
  let busy = false;
  const run = async (steps, nom) => {
    if (busy) return say("Une scène est déjà en cours", true);
    busy = true;
    try {
      for (const s of steps) { if (s.wait) await pause(s.wait); else { await press(s.k); await pause(350); } }
      say(nom ? `Scène « ${nom} » terminée` : "");
    } finally { busy = false; }
  };
  const draw = () => {
    $("#scenes").replaceChildren(...list.map((x) => {
      const b = document.createElement("button"); b.className = "b pill"; b.textContent = x.l; b.title = x.s;
      b.onclick = () => run(parse(x.s), x.l); return b;
    }));
    $("#scenes").hidden = !list.length;
    $("#sc-list").replaceChildren(...list.map((x, i) => {
      const r = document.createElement("div"); r.className = "row";
      const t = document.createElement("span"); t.textContent = `${x.l}  →  ${x.s}`;
      const d = document.createElement("button"); d.className = "b pill"; d.textContent = "Retirer"; d.onclick = () => { list.splice(i, 1); persist(); };
      r.append(t, d); return r;
    }));
  };
  const persist = () => { save("scenes", JSON.stringify(list)); draw(); };
  const add = (l, s) => {
    l = l.trim().slice(0, 14); s = s.trim().slice(0, 200);
    if (!l || !parse(s).length) return say("Indique un nom et au moins une touche", true);
    if (list.length >= 12) return say("12 scènes maximum", true);
    list.push({ l, s }); persist();
  };
  $("#sc-sugg").replaceChildren(...PRESETS.map(([l, s]) => { const c = document.createElement("button"); c.type = "button"; c.className = "chip"; c.textContent = l; c.onclick = () => add(l, s); return c; }));
  $("#sc-add").onclick = () => { add($("#sc-label").value, $("#sc-steps").value); $("#sc-label").value = ""; $("#sc-steps").value = ""; };
  draw();

  // Chaîne directe : chiffres puis OK
  const go = async () => {
    const n = $("#chan").value.replace(/\D/g, "").slice(0, 4); if (!n) return;
    $("#chan").value = "";
    await run([...n].map((k) => ({ k })).concat([{ k: "ok" }]), "");
    say(`Chaîne ${n}`);
  };
  $("#chan-go").onclick = go;
  $("#chan").addEventListener("keydown", (e) => { if (e.key === "Enter") { e.preventDefault(); go(); } e.stopPropagation(); });
}
