"""Turn a netlist into a build slot: blueprint, description.json and meta.json, with numbers from the verifier.

    python tools/bygg-logic-sim/make_build.py <netlist.json> <family> <size> <profile> --source search --authors a,b
                               [--notes "..."] [--force]

Writes circuits/<family>/builds/<size>/<profile>/. Refuses a wrong circuit. If the slot already holds a
build, the new one must win by the profile's rule (README, "Builds"); then the version is bumped: major when
the ticks changed, minor otherwise. The slot's localId is kept. --force replaces the build anyway, as a major
version, for when the pins changed.
"""
import argparse
import datetime
import json
import sys
import uuid
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import netlist
import verify

REPO = Path(__file__).resolve().parents[2]
LINK = "https://github.com/byggmesterPRO/pareto-circuits"


def score(profile, gates, ticks):
    return {"fast": (ticks, gates), "small": (gates, ticks), "efficient": (gates * ticks, ticks)}[profile]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("netlist")
    ap.add_argument("family")
    ap.add_argument("size")
    ap.add_argument("profile", choices=["fast", "small", "efficient"])
    ap.add_argument("--source", default="search", choices=["generator", "search", "manual"])
    ap.add_argument("--authors", default="")
    ap.add_argument("--notes", default="")
    ap.add_argument("--force", action="store_true")
    a = ap.parse_args()

    net = netlist.load(a.netlist)
    res = verify.check(net, a.family, a.size)
    if res["problems"] or res["wrong"]:
        sys.exit(f"not a correct {a.family} {a.size}: {res}")
    slot = REPO / "circuits" / a.family / "builds" / a.size / a.profile
    meta_path, desc_path = slot / "meta.json", slot / "blueprint" / "description.json"
    version, local_id, workshop = "1.0", str(uuid.uuid4()), ""
    if meta_path.exists():
        old = json.loads(meta_path.read_text(encoding="utf-8"))
        local_id = json.loads(desc_path.read_text(encoding="utf-8"))["localId"]
        workshop = old.get("workshop", "")
        new_s, old_s = score(a.profile, res["gates"], res["ticks"]), score(a.profile, old["gates"], old["ticks"])
        if new_s >= old_s and not a.force:
            sys.exit(f"does not beat the current build: {res['gates']} gates, {res['ticks']} ticks "
                     f"against {old['gates']} gates, {old['ticks']} ticks")
        major, minor = (int(v) for v in old["version"].split("."))
        version = f"{major + 1}.0" if a.force or res["ticks"] != old["ticks"] else f"{major}.{minor + 1}"

    bp, pins = netlist.to_blueprint(net)
    (slot / "blueprint").mkdir(parents=True, exist_ok=True)
    (slot / "blueprint" / "blueprint.json").write_text(json.dumps(bp, separators=(",", ":")), encoding="utf-8")
    title = f"Pareto {a.family.replace('-', ' ').title()} {a.size} {a.profile.title()}"
    desc_path.write_text(json.dumps({
        "description": f"v{version} | {res['gates']} gates | {res['ticks']} ticks\n{LINK}",
        "localId": local_id, "name": title, "type": "Blueprint", "version": 0}, indent=4) + "\n", encoding="utf-8")
    meta = {"family": a.family, "size": a.size, "profile": a.profile, "version": version,
            "gates": res["gates"], "ticks": res["ticks"], "source": a.source, "verified": res["verified"],
            "optimal": "best-known", "authors": [x for x in a.authors.split(",") if x],
            "date": datetime.date.today().isoformat(), "workshop": workshop, "notes": a.notes, "pins": pins}
    meta_path.write_text(json.dumps(meta, indent=2) + "\n", encoding="utf-8")
    check = verify.check_slot(slot)
    if check["problems"] or check["wrong"]:
        sys.exit(f"the written slot fails the verifier: {check}")
    print(f"{title} v{version}: {res['gates']} gates, {res['ticks']} ticks -> {slot.relative_to(REPO)}")


if __name__ == "__main__":
    main()
