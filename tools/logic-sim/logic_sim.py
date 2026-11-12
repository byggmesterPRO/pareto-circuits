"""Python client for logic-sim. Keeps one logic-sim process running, so a call costs no start-up.

    from logic_sim import LogicSim

    sim = LogicSim()
    sim.verify("circuits/double-dabble/builds/12bit/fast")          # a build slot
    sim.verify(netlist, family="double-dabble", size="12bit")       # a netlist dict or a path
    sim.check(netlist, table=[f(x) for x in range(1 << bits)])      # any circuit: one output value per input
    sim.check(netlist, cases=[(x, f(x)) for x in some_inputs])      # chosen inputs
    sim.check(netlist, reference=other_netlist)                     # two circuits must agree
    sim.count(netlist), sim.delay(netlist), sim.validate("blueprint.json")
    sim.run("blueprint.json", "set #3 1\\ntick 2\\ncheck #7 1")       # a test script, for circuits with memory
    sim.stream(netlist)                                             # new inputs every tick: does it keep up
    sim.balance(netlist, ticks=None, out=None)                      # add timers until every path is the same

Every call returns the reply as a dict. reply["problems"] is empty when all is well.
A circuit is a path (build slot, blueprint.json, netlist.json) or the netlist or blueprint as a dict.
A bare blueprint needs pins={"inputs": [controller id, ...], "outputs": [controller id or None, ...]}.

Build the program first: cargo build --release, in the repo. Other languages can talk to
`logic-sim serve` the same way: one JSON request per line in, one JSON reply per line out.
"""
import json
import os
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent


def _big(v):
    """JSON numbers lose precision above 2**53, so larger values travel as strings."""
    return str(v) if isinstance(v, int) and not isinstance(v, bool) and v >= 1 << 53 else v


class LogicSim:
    def __init__(self, exe=None):
        exe = exe or os.environ.get("LOGIC_SIM") or HERE.parents[1] / "target" / "release" / ("logic-sim.exe" if sys.platform == "win32" else "logic-sim")
        if not Path(exe).exists():
            raise FileNotFoundError(f"{exe} not found. Build it with: cargo build --release (in {HERE.parents[1]})")
        self.proc = subprocess.Popen([str(exe), "serve"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True, bufsize=1)

    def request(self, cmd, circuit, **fields):
        req = {"cmd": cmd, "circuit": str(circuit) if isinstance(circuit, Path) else circuit}
        req.update({k: (str(v) if isinstance(v, Path) else v) for k, v in fields.items() if v is not None})
        self.proc.stdin.write(json.dumps(req, separators=(",", ":")) + "\n")
        self.proc.stdin.flush()
        line = self.proc.stdout.readline()
        if not line:
            raise RuntimeError("logic-sim stopped")
        return json.loads(line)

    def verify(self, circuit, family=None, size=None, pins=None):
        return self.request("verify", circuit, family=family, size=size, pins=pins)

    def check(self, circuit, table=None, cases=None, reference=None, pins=None, reference_pins=None):
        if table is not None:
            table = [_big(v) for v in table]
        if cases is not None:
            cases = [[_big(x), _big(want)] for x, want in cases]
        return self.request("check", circuit, table=table, cases=cases, reference=reference, pins=pins, reference_pins=reference_pins)

    def count(self, circuit, pins=None):
        return self.request("count", circuit, pins=pins)

    def delay(self, circuit, jumps=200_000, pins=None):
        return self.request("delay", circuit, jumps=jumps, pins=pins)

    def stream(self, circuit, steps=2000, every=1, pins=None):
        """New inputs every `every` ticks: are the outputs right, `ticks` ticks after each."""
        return self.request("stream", circuit, steps=steps, every=every, pins=pins)

    def balance(self, circuit, ticks=None, out=None, pins=None):
        """The balanced netlist is reply["netlist"], or goes to the file `out`."""
        return self.request("balance", circuit, ticks=ticks, out=out, pins=pins)

    def validate(self, circuit, pins=None):
        return self.request("validate", circuit, pins=pins)

    def run(self, circuit, script, pins=None):
        return self.request("run", circuit, script=script, pins=pins)

    def close(self):
        if self.proc.poll() is None:
            self.proc.stdin.close()
            self.proc.wait()

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
