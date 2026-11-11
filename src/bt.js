// Télécommande Bluetooth (Android 9+) : le téléphone se comporte comme un clavier/télécommande HID.
// Marche avec Android TV, Fire TV et les TV qui acceptent un accessoire Bluetooth. Pont Kotlin `ForgeBT`.
const bt = () => window.ForgeBT;
/** Touches gérées en HID (doit refléter la table de BtBridge.kt). */
export const BT_KEYS = ["up", "down", "left", "right", "ok", "exit", "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "power", "home", "back", "menu", "vol_up", "vol_down", "mute", "ch_up", "ch_down", "info", "guide", "play", "pause", "stop", "rew", "ff", "search", "red", "green", "blue", "yellow", "subtitle", "aspect", "mode_3d", "prev_ch", "tv", "fav", "rec", "apps"];
const pause = (ms) => new Promise((r) => setTimeout(r, ms));
const lire = (k) => { try { return localStorage.getItem(k); } catch { return null; } };

export const btDispo = () => !!bt()?.available?.();
export const btAppareils = () => { try { return JSON.parse(bt().devices()); } catch { return []; } };
export const btVisible = () => bt()?.discoverable?.();
export const btAdresse = (id) => lire(`bt:${id}`);

/** Envoie une touche à la TV Bluetooth du profil `id` (se connecte au besoin). */
export async function btSend(id, key) {
  if (!bt()) throw new Error("Bluetooth : disponible uniquement sur Android 9 ou plus");
  if (!bt().available()) throw new Error("Bluetooth : active-le et accepte l'autorisation « appareils à proximité », puis réessaie");
  if (bt().status() !== "connected") {
    const adr = btAdresse(id);
    if (!adr) throw new Error("TV Bluetooth non choisie : modifie le profil et sélectionne-la");
    for (let i = 0; i < 20 && bt().status() === "starting"; i++) await pause(150);
    bt().connect(adr);
    for (let i = 0; i < 30 && bt().status() !== "connected"; i++) await pause(150);
    if (bt().status() !== "connected") throw new Error("Connexion Bluetooth impossible : TV allumée, téléphone appairé avec elle, Bluetooth de la TV activé ?");
  }
  if (!bt().send(key)) throw new Error("Touche non disponible en Bluetooth");
}

/** Tape un texte dans le champ actif de la TV Bluetooth (clavier HID). */
export async function btText(id, text) {
  await btSend(id, "ok").catch(() => {}); // connecte au besoin (OK sans effet sur un champ de texte vide)
  if (!bt()?.sendText?.(text)) throw new Error("Saisie Bluetooth impossible : TV non connectée");
}
