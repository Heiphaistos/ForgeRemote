// Compare nos encodeurs infrarouge à des captures de vraies télécommandes (tests du firmware Flipper).
// Usage : node tools/ir-vectors.mjs   (télécharge les vecteurs, rien n'est conservé)
import { encode, hexLE, PROTOCOLES } from "../src/ir-enc.js";

const BASE = "https://raw.githubusercontent.com/flipperdevices/flipperzero-firmware/dev/applications/debug/unit_tests/resources/unit_tests/infrared/";
const FILES = ["nec", "necext", "samsung32", "sirc", "rc5", "rc6", "kaseikyo"];
let ok = 0, ko = 0, ignores = 0;

for (const f of FILES) {
  const txt = await (await fetch(`${BASE}test_${f}.irtest`)).text();
  const raws = {}, exps = {};
  let last = null;
  for (const b of txt.split(/\n#\n/)) {
    let m = b.match(/name: decoder_input(\d+)\ntype: raw[\s\S]*?data: ([\d ]+)/);
    if (m) raws[m[1]] = m[2].trim().split(/\s+/).map(Number).slice(1); // 1er = silence de tête
    m = b.match(/name: decoder_expected(\d+)/);
    if (m) last = m[1];
    m = b.match(/protocol: (\w+)\naddress: (.*)\ncommand: (.*)/);
    if (m && last) { exps[last] = { proto: m[1], addr: hexLE(m[2]), cmd: hexLE(m[3]) }; last = null; }
  }
  for (const k of Object.keys(raws)) {
    const e = exps[k], raw = raws[k];
    if (!e || !PROTOCOLES.includes(e.proto)) { ignores++; continue; }
    // On compare la première trame (jusqu'au premier long silence : les captures enchaînent des
    // répétitions ou d'autres commandes). Le bit de bascule RC5/RC6 varie d'une capture à l'autre : 0 ou 1 accepté.
    const cut = (a) => { const i = a.findIndex((x) => x >= 10000); return i < 0 ? a : a.slice(0, i); };
    const essais = [0, 1].map((t) => {
      const mine = cut(encode(e.proto, e.addr, e.cmd, t).pattern), r = cut(raw), bad = [];
      for (let i = 0; i < Math.min(mine.length, r.length); i++) if (Math.abs(mine[i] - r[i]) > Math.max(0.3 * r[i], 220)) bad.push(`#${i}: ${mine[i]} vs ${r[i]}`);
      if (Math.abs(mine.length - r.length) > 1) bad.push(`longueur ${mine.length} vs ${r.length}`);
      return bad;
    });
    const bad = essais.find((b) => !b.length) ?? essais[0];
    if (bad.length) { ko++; console.log(`ÉCHEC ${f}/${k} ${e.proto} a=${e.addr.toString(16)} c=${e.cmd.toString(16)}`, bad.slice(0, 4).join(" ; ")); }
    else ok++;
  }
}
console.log(`vecteurs : ${ok} conformes, ${ko} en échec, ${ignores} ignorés`);
process.exit(ko ? 1 : 0);
