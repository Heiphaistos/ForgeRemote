// Encodeurs infrarouge : (protocole, adresse, commande) -> { freq, pattern } en microsecondes
// (impulsion, silence, impulsion, ...). Valeurs adresse/commande au format de la base Flipper-IRDB (CC0).
// Vérifiés contre des captures de vraies télécommandes : node tools/ir-vectors.mjs
const bits = (v, n) => Array.from({ length: n }, (_, i) => (v >>> i) & 1); // LSB d'abord
const msb = (v, n) => bits(v, n).reverse();
const inv = (v, n) => ~v & ((1 << n) - 1);

/** Distance codée : en-tête, bits (impulsion fixe, silence 0/1), impulsion d'arrêt. */
function distance(head, mark, s0, s1, bitList, freq) {
  const p = [...head];
  for (const b of bitList) p.push(mark, b ? s1 : s0);
  p.push(mark);
  return { freq, pattern: p };
}

/** Biphasé (RC5/RC6) : suite de demi-bits [niveau, durée] fusionnés en impulsions/silences. */
function biphase(halves) {
  const p = []; let level = null;
  for (const [l, d] of halves) {
    if (l === level) p[p.length - 1] += d;
    else { if (p.length === 0 && !l) { level = l; continue; } p.push(d); level = l; } // pas de silence initial
  }
  return p;
}

const ENC = {
  NEC: (a, c) => distance([9000, 4500], 560, 560, 1690, [...bits(a & 255, 8), ...bits(inv(a & 255, 8), 8), ...bits(c & 255, 8), ...bits(inv(c & 255, 8), 8)], 38000),
  NECext: (a, c) => distance([9000, 4500], 560, 560, 1690, [...bits(a & 255, 8), ...bits((a >> 8) & 255, 8), ...bits(c & 255, 8), ...bits((c >> 8) & 255, 8)], 38000),
  Samsung32: (a, c) => distance([4500, 4500], 550, 550, 1650, [...bits(a & 255, 8), ...bits(a & 255, 8), ...bits(c & 255, 8), ...bits(inv(c & 255, 8), 8)], 38000),
  Kaseikyo: (a, c) => {
    // adresse = [genre1<<4 | genre2, vendeur bas, vendeur haut] ; commande 10 bits ; parités vérifiées sur captures réelles.
    const g12 = a & 255, v0 = (a >> 8) & 255, v1 = (a >> 16) & 255, x = v0 ^ v1;
    const b2 = (g12 & 0xf0) | (((x >> 4) ^ x) & 0xf), b3 = ((c & 0xf) << 4) | (g12 & 0xf), b4 = (c >> 4) & 255, b5 = b2 ^ b3 ^ b4;
    return distance([3360, 1665], 432, 432, 1296, [v0, v1, b2, b3, b4, b5].flatMap((b) => bits(b, 8)), 37000);
  },
};

function sirc(addrBits) {
  return (a, c) => {
    const f = [2400, 600];
    for (const b of [...bits(c & 127, 7), ...bits(a, addrBits)]) f.push(b ? 1200 : 600, 600);
    f.pop(); // la trame se termine par une impulsion
    const len = f.reduce((s, x) => s + x, 0), p = [];
    for (let r = 0; r < 3; r++) { p.push(...f); if (r < 2) p.push(45000 - len); } // trames espacées de 45 ms
    return { freq: 40000, pattern: p };
  };
}
ENC.SIRC = sirc(5); ENC.SIRC15 = sirc(8); ENC.SIRC20 = sirc(13);

/** RC5 : 889 µs / demi-bit ; 1 = silence puis impulsion. t = bit de bascule (à alterner à chaque appui).
 *  RC5X non géré : sa convention ne peut pas être confirmée par les captures disponibles. */
ENC.RC5 = (a, c, t = 0) => {
  const seq = [1, 1, t & 1, ...msb(a & 31, 5), ...msb(c & 63, 6)], h = [];
  for (const b of seq) h.push(b ? [0, 889] : [1, 889], b ? [1, 889] : [0, 889]);
  const p = biphase(h);
  if (p.length % 2 === 0) p.pop(); // finit sur une impulsion
  return { freq: 36000, pattern: p };
};

/** RC6 mode 0 : 444 µs ; 1 = impulsion puis silence ; bit de bascule double durée. */
ENC.RC6 = (a, c, t = 0) => {
  const h = [[1, 2664], [0, 888]], u = 444;
  const bit = (b, k = 1) => (b ? h.push([1, u * k], [0, u * k]) : h.push([0, u * k], [1, u * k]));
  bit(1); bit(0); bit(0); bit(0); bit(t & 1, 2); // start, mode 000, bascule
  for (const b of [...msb(a & 255, 8), ...msb(c & 255, 8)]) bit(b);
  const p = biphase(h);
  if (p.length % 2 === 0) p.pop(); // finit sur une impulsion
  return { freq: 36000, pattern: p };
};

export const PROTOCOLES = Object.keys(ENC);
export const encode = (proto, addr, cmd, toggle = 0) => (ENC[proto] ? ENC[proto](addr, cmd, toggle) : null);

/** "04 FB 00 00" (octets, petit-boutiste, format Flipper) -> entier. */
export const hexLE = (s) => s.trim().split(/\s+/).reduce((v, b, i) => v + parseInt(b, 16) * 2 ** (8 * i), 0);
