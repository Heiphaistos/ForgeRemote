# ForgeRemote

Télécommande universelle de TV (Android, Windows, Linux). Tauri 2 : cœur Rust + interface HTML/JS.

## Marques
| Marque | Voie | Appairage |
|---|---|---|
| Samsung (Tizen 2016+) | Wi-Fi, wss 8002 | confirmation sur la TV, jeton mémorisé |
| LG webOS | Wi-Fi, SSAP + socket pointeur | confirmation sur la TV, clé mémorisée |
| Panasonic Viera | Wi-Fi, SOAP 55000 | aucun (modèles avant ~2018) |
| Roku | Wi-Fi, ECP 8060 | aucun |
| Philips JointSpace v6 | Wi-Fi, HTTP 1925 | aucun (sans authentification) |
| Sony Bravia | Wi-Fi, IRCC-IP | clé PSK réglée sur la TV ; les codes sont lus sur la TV |
| 95 marques (Samsung, LG, Sony, Panasonic, Philips, Sharp, Toshiba, Hisense, TCL, Vizio…) | Infrarouge, émetteur du téléphone Android | aucun |
| Android TV, Fire TV, TV avec Bluetooth | Bluetooth : le téléphone se fait passer pour une télécommande/clavier (Android 9+) | appairage Bluetooth entre la TV et le téléphone |

Android TV / Google TV : pas de protocole Wi-Fi propriétaire (appairage chiffré non géré) ; utilise le Bluetooth ou l'infrarouge.

Codes infrarouge : base Flipper-IRDB (CC0), régénérables avec `tools/fetch_irdb.py` puis `tools/gen_ir_db.py` ; encodeurs vérifiés avec `node tools/ir-vectors.mjs`.

## Sécurité
Seules les TV ajoutées à la liste sont contactées (IP privée obligatoire). « Chercher sur le réseau » liste les TV de
marques gérées, sans rien contrôler. Les jetons restent dans le dossier de données de l'app, jamais dans l'interface.

## Développement
```
npm install
npm run tauri dev
cd src-tauri && cargo test && cd .. && node src/ir.test.mjs
npm run tauri build -- --bundles nsis          # Windows
npm run tauri android build -- --apk --debug   # Android
```
Linux (deb, rpm, AppImage) : `.github/workflows/release.yml` (tag `vX.Y.Z`), ou en local avec les paquets webkit2gtk-4.1.

## Mise à jour automatique
Au démarrage puis toutes les 6 h (désactivable) : dernière release GitHub, SHA-256 vérifié via `SHA256SUMS`
avant installation silencieuse (Windows). Android/Linux : lien vers la release.
