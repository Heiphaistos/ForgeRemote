"""Génère src/ir-db.js (codes infrarouge TV par marque) depuis les fichiers de tools/fetch_irdb.py.
Source : Flipper-IRDB (CC0). Pour chaque marque on garde le fichier qui couvre le plus de touches
avec des protocoles qu'on sait encoder (vérifiés par tools/ir-vectors.mjs).
Usage : python tools/gen_ir_db.py <dossier_irdb> src/ir-db.js"""
import re, sys
from collections import defaultdict
from pathlib import Path

SUPPORTES = {"NEC", "NECext", "Samsung32", "SIRC", "SIRC15", "SIRC20", "RC5", "RC6", "Kaseikyo"}
ALIAS = {
    "power": ["power", "pwr", "standby", "onoff"], "mute": ["mute"],
    "vol_up": ["volup", "volumeup", "vol+"], "vol_down": ["voldn", "voldown", "volumedown", "vol-"],
    "ch_up": ["chnext", "chup", "chanup", "channelup", "ch+"], "ch_down": ["chprev", "chdown", "chandown", "channeldown", "ch-"],
    "up": ["up"], "down": ["down"], "left": ["left"], "right": ["right"], "ok": ["ok", "enter", "select"],
    "back": ["back", "return"], "home": ["home"], "menu": ["menu"], "exit": ["exit"],
    "source": ["source", "input"], "info": ["info", "display"], "guide": ["guide", "epg"],
    "play": ["play"], "pause": ["pause"], "stop": ["stop"], "rew": ["rewind", "fastba", "rew"], "ff": ["fastforward", "fastfo", "ff"],
    "red": ["red"], "green": ["green"], "yellow": ["yellow"], "blue": ["blue"],
}
for d in "0123456789":
    ALIAS[d] = [d, "num" + d, "digit" + d]
CLE = {a: k for k, v in ALIAS.items() for a in v}


def hex_le(s):
    return sum(int(b, 16) << (8 * i) for i, b in enumerate(s.split()))


def lire(fichier):
    touches = {}
    for bloc in fichier.read_text(encoding="utf-8", errors="ignore").split("#"):
        m = re.search(r"name: (.*)\ntype: parsed\nprotocol: (\w+)\naddress: (.*)\ncommand: (.*)", bloc)
        if not m or m.group(2) not in SUPPORTES:
            continue
        cle = CLE.get(re.sub(r"[^a-z0-9+\-]", "", m.group(1).lower().replace("_", "")))
        if cle and cle not in touches:
            touches[cle] = [m.group(2), hex_le(m.group(3)), hex_le(m.group(4))]
    return touches


marques = defaultdict(list)
for f in sorted(Path(sys.argv[1]).glob("*.ir")):
    parts = f.name.split("__")  # TVs__Marque__[Modele__]fichier.ir
    marques[parts[1]].append(f)

db, proto = {}, defaultdict(int)
for nom, fichiers in sorted(marques.items(), key=lambda x: x[0].lower()):
    meilleur = max((lire(f) for f in fichiers), key=len)
    if len(meilleur) >= 8 and ("power" in meilleur or "vol_up" in meilleur):
        db[re.sub(r"[^a-z0-9]", "", nom.lower())] = (nom.replace("_", " "), meilleur)
        for v in meilleur.values():
            proto[v[0]] += 1

lignes = ["// GÉNÉRÉ par tools/gen_ir_db.py depuis Flipper-IRDB (CC0) : ne pas modifier à la main.",
          "// marque -> { name, keys: { touche: [protocole, adresse, commande] } }", "export default {"]
for slug, (nom, touches) in db.items():
    ks = ", ".join(f'"{k}": ["{p}", {a}, {c}]' for k, (p, a, c) in touches.items())
    lignes.append(f'  {slug}: {{ name: "{nom}", keys: {{ {ks} }} }},')
lignes.append("};")
Path(sys.argv[2]).write_text("\n".join(lignes) + "\n", encoding="utf-8")
print(len(db), "marques,", sum(len(v[1]) for v in db.values()), "touches ;", dict(proto))
