// node src/ir.test.mjs : cohérence de la base et des trames (la conformité aux vraies télécommandes
// est vérifiée par tools/ir-vectors.mjs).
import assert from "node:assert";
import DB from "./ir-db.js";
import { encode, PROTOCOLES } from "./ir-enc.js";

assert.ok(Object.keys(DB).length >= 50, "au moins 50 marques");
assert.deepStrictEqual(DB.samsung.keys.power, ["Samsung32", 7, 2]); // codes connus, concordent avec la base
assert.deepStrictEqual(DB.lg.keys.power, ["NEC", 4, 8]);
assert.deepStrictEqual(DB.sony.keys.power, ["SIRC", 1, 21]);
assert.ok(DB.panasonic.keys.power[0] === "Kaseikyo", "Panasonic en Kaseikyo");

let n = 0;
for (const [marque, { keys }] of Object.entries(DB)) {
  for (const [touche, [proto, a, c]] of Object.entries(keys)) {
    assert.ok(PROTOCOLES.includes(proto), `${marque}/${touche} : protocole ${proto} non encodable`);
    for (const t of [0, 1]) {
      const { freq, pattern } = encode(proto, a, c, t);
      assert.ok(freq >= 30000 && freq <= 60000, `${marque}/${touche} fréquence`);
      assert.ok(pattern.length % 2 === 1 && pattern.every((x) => Number.isFinite(x) && x > 0), `${marque}/${touche} : trame invalide`);
      n++;
    }
  }
}
// la bascule RC5/RC6 change la trame, pas les autres protocoles
assert.notDeepStrictEqual(encode("RC5", 5, 12, 0).pattern, encode("RC5", 5, 12, 1).pattern);
assert.deepStrictEqual(encode("NEC", 4, 8, 0).pattern, encode("NEC", 4, 8, 1).pattern);
console.log(`IR OK (${n} trames de ${Object.keys(DB).length} marques)`);
