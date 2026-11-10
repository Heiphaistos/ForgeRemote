"""Télécharge un échantillon de la base Flipper-IRDB (licence CC0) pour les TV.
Usage : python tools/fetch_irdb.py <dossier_sortie> [fichiers_max_par_marque]
Pour chaque marque : les plus gros fichiers .ir (plus de boutons) sont gardés."""
import json, subprocess, sys, urllib.parse, urllib.request
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

REPO = "Lucaslhm/Flipper-IRDB"
out = Path(sys.argv[1]); out.mkdir(parents=True, exist_ok=True)
per_brand = int(sys.argv[2]) if len(sys.argv) > 2 else 12


def gh(path):
    r = subprocess.run(["gh", "api", path], capture_output=True, text=True, encoding="utf-8")
    if r.returncode:
        raise RuntimeError(r.stderr)
    return json.loads(r.stdout)


def brand_files(b):
    # Les marques ont soit des .ir directement, soit des sous-dossiers de modèles.
    items = gh(f"repos/{REPO}/contents/TVs/{urllib.parse.quote(b)}")
    files = [i for i in items if i["name"].endswith(".ir")]
    for d in [i for i in items if i["type"] == "dir"][:6]:
        files += [i for i in gh(f"repos/{REPO}/contents/{urllib.parse.quote(d['path'])}") if i["name"].endswith(".ir")]
    files.sort(key=lambda i: -i["size"])
    return files[:per_brand]


def grab(item):
    dest = out / item["path"].replace("/", "__")
    if not dest.exists():
        with urllib.request.urlopen(item["download_url"], timeout=30) as r:
            dest.write_bytes(r.read())
    return dest.name


brands = [i["name"] for i in gh(f"repos/{REPO}/contents/TVs") if i["type"] == "dir"]
todo = []
for b in brands:
    try:
        todo += brand_files(b)
    except Exception as e:  # une marque en échec ne bloque pas les autres
        print("ignorée", b, str(e)[:80])
with ThreadPoolExecutor(8) as ex:
    names = list(ex.map(grab, todo))
print(len(brands), "marques,", len(names), "fichiers dans", out)
