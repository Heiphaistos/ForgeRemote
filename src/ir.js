// Infrarouge Android (ConsumerIrManager via le pont Kotlin `ForgeIR`). Encodeurs NEC, Samsung32, SIRC12.
// Les tables de codes sont les codes TV courants de chaque marque : à valider sur ta TV.
const D = "0123456789";
const num = (base, order) => Object.fromEntries([...D].map((d, i) => [d, order ? order[i] : base + i]));

const TABLES = {
  ir_samsung: { proto: "samsung32", addr: 0x07, keys: {
    power: 0x02, ...num(0, [0x11, 0x04, 0x05, 0x06, 0x08, 0x09, 0x0a, 0x0c, 0x0d, 0x0e]),
    up: 0x60, down: 0x61, left: 0x65, right: 0x62, ok: 0x68, back: 0x58, home: 0x79, menu: 0x1a, exit: 0x2d,
    vol_up: 0x07, vol_down: 0x0b, mute: 0x0f, ch_up: 0x12, ch_down: 0x10, source: 0x01, info: 0x1f, guide: 0x4f,
    play: 0x47, pause: 0x4a, stop: 0x46, rew: 0x45, ff: 0x48, red: 0x6c, green: 0x14, yellow: 0x15, blue: 0x16 } },
  ir_lg: { proto: "nec", addr: 0x04, keys: {
    power: 0x08, ...num(0x10), up: 0x40, down: 0x41, left: 0x07, right: 0x06, ok: 0x44, back: 0x28, home: 0x7c, menu: 0x43, exit: 0x5b,
    vol_up: 0x02, vol_down: 0x03, mute: 0x09, ch_up: 0x00, ch_down: 0x01, source: 0x0b, info: 0xaa, guide: 0xa9,
    play: 0xb0, pause: 0xba, stop: 0xb1, rew: 0x8f, ff: 0x8e, red: 0x72, green: 0x71, yellow: 0x63, blue: 0x61 } },
  ir_sony: { proto: "sirc12", addr: 1, keys: {
    power: 21, ...num(0, [9, 0, 1, 2, 3, 4, 5, 6, 7, 8]), up: 116, down: 117, left: 52, right: 51, ok: 101, back: 98, home: 96,
    vol_up: 18, vol_down: 19, mute: 20, ch_up: 16, ch_down: 17, source: 37, info: 58, play: 26, pause: 25, stop: 24, rew: 27, ff: 28 } },
};

const bits = (v, n) => Array.from({ length: n }, (_, i) => (v >> i) & 1); // LSB d'abord

function nec(addr, cmd, samsung) {
  const frame = samsung ? [...bits(addr, 8), ...bits(addr, 8)] : [...bits(addr, 8), ...bits(~addr & 255, 8)];
  frame.push(...bits(cmd, 8), ...bits(~cmd & 255, 8));
  const p = samsung ? [4500, 4500] : [9000, 4500];
  frame.forEach((b) => p.push(560, b ? 1690 : 560));
  p.push(560); // bit d'arrêt
  return { freq: 38000, pattern: p };
}

function sirc(addr, cmd) {
  const one = [2400, 600];
  for (const b of [...bits(cmd, 7), ...bits(addr, 5)]) one.push(b ? 1200 : 600, 600);
  one.pop(); one.push(0); // pas de silence final : on l'ajoute entre répétitions
  const p = [];
  for (let r = 0; r < 3; r++) { p.push(...one.slice(0, -1)); if (r < 2) p.push(25000); } // ponytail: 3 trames, espacement fixe
  return { freq: 40000, pattern: p };
}

export const irDispo = () => !!window.ForgeIR?.hasIr?.();

export function irSend(brand, key) {
  if (!window.ForgeIR) throw new Error("Infrarouge disponible uniquement sur Android");
  if (!irDispo()) throw new Error("Ce téléphone n'a pas d'émetteur infrarouge");
  const t = TABLES[brand], c = t?.keys[key];
  if (c === undefined) throw new Error("Touche non disponible en infrarouge pour cette marque");
  const { freq, pattern } = t.proto === "sirc12" ? sirc(t.addr, c) : nec(t.addr, c, t.proto === "samsung32");
  window.ForgeIR.transmit(freq, JSON.stringify(pattern));
}

export { nec, sirc }; // pour le test
