"""Post builds to the Steam Workshop with SteamCMD, one item per build.

    python tools/workshop/upload.py --user <steam login> --dry-run            show what would be posted
    python tools/workshop/upload.py --user <steam login> --limit 10           post the first 10 that are not posted yet
    python tools/workshop/upload.py --user <steam login> adder ram            only these families
    python tools/workshop/upload.py --user <steam login> --public             post as public (default is hidden)
    python tools/workshop/upload.py --user <steam login> --update             also re-upload builds that are already posted

Run `python tools/workshop/cards.py` first: this posts the description and icon.png that are in each slot.
Only fast and small builds are posted, unless --efficient is given.

What it does for each build: writes a SteamCMD item file, runs `steamcmd +login <user> +workshop_build_item`,
reads the item id Steam gives back, and stores it in the slot (meta.json `workshop`, description.json `fileId`)
so the next run updates that item instead of making a new one. One login for the whole run, a pause between
items, and it stops at the first failure. Run it again to carry on where it stopped.

SteamCMD has to be installed (--steamcmd <path> if it is not on PATH), and the Steam account has to own Scrap
Mechanic. The first login asks for the password and Steam Guard code in the terminal; SteamCMD remembers it.

Not known yet: whether an item posted this way shows up as a blueprint in the game without the tags the game
sets itself. Post one hidden item and check before posting many.
"""
import json
import re
import shutil
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNS = Path(__file__).resolve().parent / "runs"
APP = 387990    # Scrap Mechanic
PAUSE = 8       # seconds between items


def vdf(text):
    return text.replace("\\", "\\\\").replace('"', '\\"')


def item_file(slot, meta, desc, public, note):
    bp = slot / "blueprint"
    fields = {
        "appid": APP,
        "publishedfileid": desc.get("fileId", 0),
        "contentfolder": str(bp),
        "previewfile": str(bp / "icon.png"),
        "visibility": 0 if public else 2,      # 0 public, 2 hidden
        "title": desc["name"],
        "description": desc["description"],
        "changenote": note,
    }
    body = "\n".join(f'\t"{k}"\t\t"{vdf(str(v))}"' for k, v in fields.items())
    return f'"workshopitem"\n{{\n{body}\n}}\n'


def main():
    args = sys.argv[1:]

    def flag(name):
        if name in args:
            args.remove(name)
            return True
        return False

    def value(name, default=None):
        if name in args:
            i = args.index(name)
            v = args[i + 1]
            del args[i:i + 2]
            return v
        return default

    dry, public, update, efficient = flag("--dry-run"), flag("--public"), flag("--update"), flag("--efficient")
    user, limit = value("--user"), int(value("--limit", "0"))
    steamcmd = value("--steamcmd") or shutil.which("steamcmd")
    families = args
    if not user and not dry:
        sys.exit("give --user <steam login>")
    if not steamcmd and not dry:
        sys.exit("steamcmd is not on PATH; give --steamcmd <path to steamcmd.exe>")

    todo = []
    for meta_path in sorted((ROOT / "circuits").glob("*/builds/*/*/meta.json")):
        meta = json.loads(meta_path.read_text())
        slot = meta_path.parent
        desc = json.loads((slot / "blueprint" / "description.json").read_text())
        if families and meta["family"] not in families:
            continue
        if meta["profile"] == "efficient" and not efficient:
            continue
        if not (slot / "blueprint" / "icon.png").exists():
            sys.exit(f"{slot} has no icon.png; run tools/workshop/cards.py first")
        if desc.get("fileId") and not update:
            continue
        todo.append((slot, meta, desc))
    if limit:
        todo = todo[:limit]
    print(len(todo), "builds to post,", "public" if public else "hidden")
    RUNS.mkdir(exist_ok=True)

    # one SteamCMD run for all of them, so there is one login; an item file per build
    for k, (slot, meta, desc) in enumerate(todo):
        name = f"{meta['family']}-{meta['size']}-{meta['profile']}"
        note = f"v{meta['version']}: {meta['gates']} gates, {meta['ticks']} ticks"
        path = RUNS / f"{name}.vdf"
        path.write_text(item_file(slot, meta, desc, public, note), encoding="utf-8")
        if dry:
            print("would post", desc["name"], "as a new item" if not desc.get("fileId") else f"to item {desc['fileId']}")
            continue
        if k:
            time.sleep(PAUSE)
        run = subprocess.run([steamcmd, "+login", user, "+workshop_build_item", str(path), "+quit"], capture_output=True, text=True)
        out = run.stdout + run.stderr
        (RUNS / f"{name}.log").write_text(out, encoding="utf-8")
        # SteamCMD writes the new id back into the item file
        found = re.search(r'"publishedfileid"\s+"(\d+)"', path.read_text(encoding="utf-8"))
        if "Success" not in out or not found or found.group(1) == "0":
            sys.exit(f"{name}: Steam did not accept it; see {RUNS / (name + '.log')}. Nothing after it was posted.")
        file_id = int(found.group(1))
        desc["fileId"] = file_id
        (slot / "blueprint" / "description.json").write_text(json.dumps(desc, indent=2) + "\n", newline="\n")
        meta["workshop"] = f"https://steamcommunity.com/sharedfiles/filedetails/?id={file_id}"
        (slot / "meta.json").write_text(json.dumps(meta, indent=2) + "\n", newline="\n")
        print(f"{k + 1}/{len(todo)} {desc['name']}: {meta['workshop']}")


if __name__ == "__main__":
    main()
