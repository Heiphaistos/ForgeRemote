"""Génère src/ir-db.js (codes infrarouge TV par marque) depuis les fichiers de tools/fetch_irdb.py.
Source : Flipper-IRDB (CC0). Pour chaque marque on garde le fichier qui couvre le plus de touches
avec des protocoles qu'on sait encoder (vérifiés par tools/ir-vectors.mjs).
Usage : python tools/gen_ir_db.py <dossier_irdb> src/ir-db.js [ancien ir-db.js à fusionner]"""
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
    "play": ["play"], "pause": ["pause"], "stop": ["stop"], "rew": ["rewind", "fastba", "rew"], "ff": ["fastforward", "fastfo", "ff", "ffw", "fwd", "forward", "fastfwd"],
    "red": ["red", "colorred", "ared", "a-red"], "green": ["green", "colorgreen", "bgreen", "b-green"],
    "yellow": ["yellow", "coloryellow", "cyellow", "c-yellow"], "blue": ["blue", "colorblue", "dblue", "d-blue"],
}
# Touches « en plus » (0.8.0) : ajoutées SANS toucher aux codes déjà présents, et seulement si elles viennent de la
# même télécommande que les codes existants de la marque (même protocole + même adresse).
EXTRA = {
    "tools": ["tools", "tool", "options", "option", "qmenu", "submenu", "vieratools", "actionmenu"],
    "prev_ch": ["flashback", "flashbk", "lastview", "last", "prevchannel", "lastchannel", "recall", "prech", "reloadlastview"],
    "ch_list": ["chlist", "list", "channellist"],
    "subtitle": ["subtitle", "subtitles", "subs", "sttl", "cc", "caption", "captions", "subt"],
    "teletext": ["teletext", "text", "txt", "ttx"],
    "aspect": ["aspect", "aspectratio", "format", "picturesize", "screensize", "size"],
    "sleep": ["sleep", "sleeptimer", "timer"],
    "rec": ["rec", "record"],
    "fav": ["fav", "favorite", "favorites", "favch"],
    "audio_desc": ["ad", "audiodescription"],
    "tv": ["tv", "dtv", "livetv"],
    "apps": ["apps", "app", "myapps", "myapp", "smarthub", "smarttv", "internet", "portal"],
    "netflix": ["netflix"], "prime": ["prime", "primevideo", "amazon"], "disney": ["disney+", "disney"], "youtube": ["youtube"],
    "search": ["search"], "replay": ["replay", "instantreplay"], "dash": ["dash", "minus", "minus-"], "mode_3d": ["3d", "3dmode"],
    "hdmi": ["hdmi"], "hdmi1": ["hdmi1"], "hdmi2": ["hdmi2"], "hdmi3": ["hdmi3"], "hdmi4": ["hdmi4"],
    "power_on": ["poweron"], "power_off": ["poweroff"],
}
CLE_EXTRA = {a: k for k, v in EXTRA.items() for a in v}
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
        brut = re.sub(r"[^a-z0-9+\-]", "", m.group(1).lower().replace("_", ""))
        cle = CLE.get(brut) or CLE_EXTRA.get(brut)
        if cle and cle not in touches:
            touches[cle] = [m.group(2), hex_le(m.group(3)), hex_le(m.group(4))]
    return touches


def lire_ancien(chemin):
    """{slug: (nom, {touche: [proto, adr, cmd]})} d'un ir-db.js existant."""
    ancien = {}
    for l in Path(chemin).read_text(encoding="utf-8").splitlines():
        m = re.match(r'  (\w+): \{ name: "(.*?)", keys: \{ (.*) \} \},', l)
        if m:
            ancien[m.group(1)] = (m.group(2), {k: [p, int(a), int(c)] for k, p, a, c in re.findall(r'"(\w+)": \["(\w+)", (\d+), (\d+)\]', m.group(3))})
    return ancien


marques = defaultdict(list)
for f in sorted(Path(sys.argv[1]).glob("*.ir")):
    parts = f.name.split("__")  # TVs__Marque__[Modele__]fichier.ir
    marques[parts[1]].append(f)

marques_par_slug = {re.sub(r"[^a-z0-9]", "", n.lower()): fs for n, fs in marques.items()}
db, proto = {}, defaultdict(int)
for nom, fichiers in sorted(marques.items(), key=lambda x: x[0].lower()):
    meilleur = max((lire(f) for f in fichiers), key=len)
    if len(meilleur) >= 4 and ("power" in meilleur or "vol_up" in meilleur):
        db[re.sub(r"[^a-z0-9]", "", nom.lower())] = (nom.replace("_", " "), meilleur)
        for v in meilleur.values():
            proto[v[0]] += 1

if len(sys.argv) > 3:  # fusion : codes existants intacts, extras ajoutés depuis la même télécommande
    ancien = lire_ancien(sys.argv[3])
    ajouts = 0
    for slug, (nom, anc) in ancien.items():
        fichiers = marques_par_slug.get(slug, [])
        majo = defaultdict(int)
        for p, a, c in anc.values():
            majo[(p, a)] += 1
        if not majo:
            continue
        ref = max(majo, key=majo.get)
        fusion = dict(anc)
        for f in fichiers:
            for k, v in lire(f).items():
                if k not in fusion and (k in EXTRA or k in ALIAS) and (v[0], v[1]) == ref:
                    fusion[k] = v
                    ajouts += 1
        db[slug] = (nom, fusion)
    print(ajouts, "touches ajoutées sans toucher aux codes existants")

lignes = ["// GÉNÉRÉ par tools/gen_ir_db.py depuis Flipper-IRDB (CC0) : ne pas modifier à la main.",
          "// marque -> { name, keys: { touche: [protocole, adresse, commande] } }", "export default {"]
for slug, (nom, touches) in db.items():
    ks = ", ".join(f'"{k}": ["{p}", {a}, {c}]' for k, (p, a, c) in touches.items())
    lignes.append(f'  {slug}: {{ name: "{nom}", keys: {{ {ks} }} }},')
lignes.append("};")
Path(sys.argv[2]).write_text("\n".join(lignes) + "\n", encoding="utf-8")
print(len(db), "marques,", sum(len(v[1]) for v in db.values()), "touches ;", dict(proto))
