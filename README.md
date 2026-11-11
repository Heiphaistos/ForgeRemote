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

## Nouveautés 0.6.0

- Profil par mode : Wi-Fi, infrarouge ou Bluetooth ; « Dupliquer » crée le même profil dans l'autre mode.
- Marche/arrêt par profil : auto, Wi-Fi seul (touche + réveil réseau) ou infrarouge seul.
- Adresse MAC apprise automatiquement (table ARP) pour réveiller la TV en veille par le réseau ; réveil envoyé sur le /24 de la TV, ports 9 et 7.
- 25 touches en plus (Outils, sous-titres, télétexte, format image, minuterie, HDMI 1 à 4, Netflix, Prime Video, Allumer, Éteindre...), en Wi-Fi.
- Mise à jour en un clic (« Installer ») : Windows installé = installeur silencieux, portable = remplacement de l'exe, Android = APK vérifié (SHA-256) puis écran système d'installation.

## Nouveautés 0.7.0

- Raccourcis d'applications personnalisables (bouton « Raccourcis » en bas) : Plex, Netflix, YouTube, Prime, Disney+ par défaut ; ajout/retrait libre (suggestions + nom perso). L'appli est retrouvée par son nom dans la liste d'applis de la TV (Samsung, LG, Roku) ; secours par identifiants Samsung connus.
- Recherche réseau dans une fenêtre au premier plan (avancement, résultat, relance, aide si la TV est éteinte).
- Noms de TV décodés (`65&quot;` -> `65"`), MAC lue dans la table ARP puis l'API de la TV.
- Correctif : le bandeau « Installer » ne s'affiche plus en permanence.

## Nouveautés 0.9.6

- Marques expérimentales (protocoles des bibliothèques publiques pyvizio, pymitv, pylgnetcast et de l'API locale Fire TV ; vérifiés seulement contre des serveurs simulés, JAMAIS sur une vraie TV) : Vizio SmartCast (appairage par code), Xiaomi Mi TV, LG Netcast 2012-2014 (appairage par code), Amazon Fire TV (appairage par code). Bouton « Afficher le code sur la TV » dans le profil. Xiaomi et Vizio sont aussi reconnus par la recherche réseau.
- LG webOS : liste des entrées (HDMI 1, AV...) lue sur la TV, un bouton par entrée (Plus de touches > Entrées de la TV).

## Nouveautés 0.9.5 (sécurité)

- Mises à jour signées : `SHA256SUMS` est signé en Ed25519 (`SHA256SUMS.sig`, clé privée hors dépôt) ; l'application vérifie la signature avec sa clé publique intégrée AVANT de se fier aux sommes SHA-256, puis vérifie le fichier. Un dépôt ou compte GitHub compromis ne suffit plus à faire installer un binaire modifié. Signature faite par `tools/sign_release.py`. La somme du dmg macOS est dans `SHA256SUMS.macos` (non signée : macOS ne s'installe pas automatiquement).
- Certificat des TV épinglé à la première connexion (TOFU) : si le certificat d'une TV change (usurpation sur le réseau local, autre appareil à la même adresse), la connexion est refusée. TV réinitialisée : Modifier > Enregistrer oublie l'ancienne empreinte.

## Nouveautés 0.9.4

- Raccourcis d'applis : « Lire les applis de la TV » (Samsung, LG, Roku) liste les applis réellement installées, un clic crée le bouton.
- Android : option « Boutons de volume du téléphone = volume de la TV » (Paramètres > Apparence).

## Nouveautés 0.9.3

- Correctif : dans la fenêtre Windows (barre de défilement comprise), les cinq boutons Modifier/Ajouter/Dupliquer/Retirer/Groupe tiennent sur une seule ligne (marges latérales réduites).

## Nouveautés 0.9.2

- Correctif : le bouton « Groupe » ne dépasse plus de la télécommande (télécommande un peu plus large, boutons d'action sur une seule ligne).

## Nouveautés 0.9.1

- Profil infrarouge + IP facultative de la TV : les boutons d'applis (Plex, Disney+, YouTube, Netflix, Prime et tes raccourcis) s'activent et se lancent par le Wi-Fi, l'infrarouge restant utilisé pour tout le reste. Modifier le profil, renseigner l'IP.
- Scènes (Paramètres > Scènes) : une suite de touches en un clic, avec attentes (ex. `power 5s hdmi1`) ; saisie directe d'un numéro de chaîne (Plus de touches).
- 14 thèmes de plus (38 au total) : Émeraude, Améthyste, Corail, Turquoise, Bordeaux, Graphite, Bronze, Arctique, Citron, Bubblegum, Matrix, Désert, Galaxie, Acier bleu.

## Nouveautés 0.9.0 (cumul des versions 0.7 à 0.9)

- Sécurité (audit interne) : liste blanche d'URL de mise à jour analysée (plus de contournement `../` / `%2e`), noms de fichiers stricts, dossier temporaire imprévisible, socket pointeur LG limité à la TV, redirections HTTP interdites vers les TV, jeton Samsung et code Sony nettoyés, sauvegarde Android désactivée (jetons TV), FileProvider restreint.
- 4 thèmes de plus (Océan, Forêt, Plex, Sakura) ; correctif du thème Clair.
- Linux : mise à jour automatique de l'AppImage ; macOS (dmg universel, non signé) et paquets Linux ajoutés à la release par le workflow GitHub Actions.
- Infrarouge : 350 codes ajoutés (Outils, HDMI 1 à 4, sous-titres, télétexte, minuterie, couleurs, Accueil... pour Samsung, LG, Sony, Panasonic, Philips et 90 autres marques), sans modifier aucun code existant (recoupés avec les codes Samsung connus). Une TV Wi-Fi sans une touche utilise l'infrarouge du téléphone en secours ; les touches qu'un profil ne gère pas sont grisées au lieu d'afficher une erreur.
- Saisie de texte vers la TV (Samsung, LG, Roku) et maintien d'une touche (volume, chaînes, flèches) pour répéter.
- Interface refaite en relief 3D (« Studio » par défaut, inclinaison selon le pointeur), 24 thèmes et 20 dispositions, tout réglable dans le bouton « Paramètres » (thèmes, dispositions, effets, raccourcis d'applis, mises à jour). « Classique » (rendu d'origine) reste disponible.
- Bandeau de mise à jour explicite (« Installer maintenant » / « Plus tard »).
- Parité des modes : Bluetooth gagne les couleurs, sous-titres, format d'image, 3D, dernière chaîne, favoris, enregistrement et la saisie de texte au clavier HID ; l'infrarouge compte 100 marques et plus de 2 500 touches.
