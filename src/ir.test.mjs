// node src/ir.test.mjs : vérifie la forme des trames infrarouge.
import assert from "node:assert";
const { nec, sirc } = await import("./ir.js");

const n = nec(0x04, 0x08, false); // LG power : en-tête + 32 bits + arrêt
assert.strictEqual(n.freq, 38000);
assert.strictEqual(n.pattern.length, 2 + 64 + 1);
assert.deepStrictEqual(n.pattern.slice(0, 2), [9000, 4500]);
// adresse 0x04 LSB d'abord = 0,0,1,0,0,0,0,0 -> espaces 560,560,1690,...
assert.deepStrictEqual(n.pattern.filter((_, i) => i >= 2 && i % 2 === 1).slice(0, 8), [560, 560, 1690, 560, 560, 560, 560, 560]);

const s = nec(0x07, 0x07, true); // Samsung32 : en-tête 4500/4500, adresse répétée
assert.deepStrictEqual(s.pattern.slice(0, 2), [4500, 4500]);
assert.strictEqual(s.pattern.length, 2 + 64 + 1);

const r = sirc(1, 21); // Sony power : 3 trames de 12 bits
assert.strictEqual(r.freq, 40000);
assert.strictEqual(r.pattern.filter((x) => x === 25000).length, 2);
assert.strictEqual(r.pattern.length % 2, 1); // impulsion/silence alternés, finit par une impulsion
assert.ok(r.pattern.every((x) => x > 0));
console.log("IR OK");
