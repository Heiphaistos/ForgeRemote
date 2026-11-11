// Infrarouge Android (ConsumerIrManager via le pont Kotlin `ForgeIR`).
// Codes : base Flipper-IRDB (CC0, src/ir-db.js) ; encodeurs vérifiés sur de vraies captures (src/ir-enc.js).
import DB from "./ir-db.js";
import { encode } from "./ir-enc.js";

/** Marques disponibles en infrarouge : [{ id: "ir_samsung", name: "Samsung" }, ...] triées par nom. */
export const irMarques = () =>
  Object.entries(DB).map(([k, v]) => ({ id: `ir_${k}`, name: v.name })).sort((a, b) => a.name.localeCompare(b.name, "fr"));

export const irDispo = () => !!window.ForgeIR?.hasIr?.();

/** Marche/arrêt d'une TV Wi-Fi depuis la veille : le Wi-Fi ne l'atteint plus, l'infrarouge si (sans appairage, sans MAC). */
export const irPowerWifi = (brand) => irDispo() && !!DB[brand]?.keys.power;

const bascule = {}; // RC5/RC6 : le bit de bascule alterne à chaque appui pour que la TV distingue deux appuis identiques

export function irSend(brand, key) {
  if (!window.ForgeIR) throw new Error("Infrarouge disponible uniquement sur Android");
  if (!irDispo()) throw new Error("Ce téléphone n'a pas d'émetteur infrarouge");
  const code = DB[brand.replace(/^ir_/, "")]?.keys[key];
  if (!code) throw new Error("Touche non disponible en infrarouge pour cette marque");
  bascule[brand] = 1 - (bascule[brand] ?? 1);
  const { freq, pattern } = encode(code[0], code[1], code[2], bascule[brand]);
  window.ForgeIR.transmit(freq, JSON.stringify(pattern));
}
