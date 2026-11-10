"""Run resub.py on every core, spread over a few start netlists, and list what each run reached.

    python many.py <family> <size> <seconds> <out folder> <start.json> [<start.json> ...] [--procs 18]

Run it again with the best results as the new starts until nothing improves. Different starting layouts end
in different places, so keep a few.
"""
import random
import re
import subprocess
import sys
from pathlib import Path

args = sys.argv[1:]
procs = 18
if "--procs" in args:
    i = args.index("--procs")
    procs = int(args[i + 1])
    del args[i:i + 2]
family, size, seconds, out = args[0], args[1], args[2], Path(args[3])
starts = args[4:]
out.mkdir(parents=True, exist_ok=True)
resub = str(Path(__file__).resolve().parent / "resub.py")
jobs = []
for p in range(procs):
    start = starts[p % len(starts)]
    dest = out / f"{Path(start).stem}_{p}.json"
    cmd = [sys.executable, resub, family, size, start, str(dest), str(random.randrange(1 << 30)), seconds]
    jobs.append((start, dest, subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)))
results = []
for start, dest, proc in jobs:
    stdout, stderr = proc.communicate()
    found = re.findall(r"best (\d+)", stdout)
    results.append((int(found[-1]) if found else 10 ** 9, dest, start, stderr.strip().split("\n")[-1]))
for gates, dest, start, err in sorted(results):
    print(f"{gates if gates < 10 ** 9 else 'failed: ' + err}  {dest}  (from {start})")
