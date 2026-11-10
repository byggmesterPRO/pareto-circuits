"""Tick-accurate simulator for Scrap Mechanic logic, straight from a blueprint.json. No dependencies.

Model (not yet checked against the game):
- The game runs logic at 40 ticks per second.
- Every gate reads its inputs' state from the previous tick, so each gate adds 1 tick of delay.
- A gate with no inputs is off.
- XOR is on when an odd number of inputs are on. NAND, NOR and XNOR are the inverses of AND, OR and XOR.
- A timer outputs its input delayed by seconds*40 + ticks, plus the 1 tick every part has.
- Switches, buttons and any part you call force() on are driven from outside. A switch starts in its saved
  `active` state.
- Lamps (any part with a light controller) are on when any input is on.
- Feedback loops are allowed; that is how memory and clocks are made.

    sim = Sim(json.load(open("blueprint.json")))
    sim.force(controller_id, True)
    sim.tick(3)
    sim.get(controller_id)
    sim.settle()        # ticks until nothing changes, -1 for a clock
"""
LOGIC_GATE = "9f0f56e8-2c31-4d83-996c-d00a9b296c3f"
TIMER = "8f7fd0e7-c46e-4944-a414-7ce2437bb30f"
SWITCH = "7cf717d7-d167-4f2d-a6e7-6b2c70aa3986"
BUTTON = "1e8d93a4-506b-470d-9ada-9c0a321e2db5"
TICKS_PER_SECOND = 40
AND, OR, XOR, NAND, NOR, XNOR = range(6)


class Sim:
    def __init__(self, blueprint):
        self.kind, self.mode, self.delay, self.initial = {}, {}, {}, {}
        self.inputs = {}
        for body in blueprint["bodies"]:
            for part in body["childs"]:
                ctrl = part.get("controller")
                if not ctrl or "id" not in ctrl:
                    continue
                pid = ctrl["id"]
                self.inputs.setdefault(pid, [])
                shape = part["shapeId"]
                if shape == LOGIC_GATE:
                    self.kind[pid], self.mode[pid] = "gate", ctrl["mode"]
                elif shape == TIMER:
                    self.kind[pid] = "timer"
                    self.delay[pid] = ctrl["seconds"] * TICKS_PER_SECOND + ctrl["ticks"]
                elif shape == SWITCH:
                    self.kind[pid] = "switch"
                    self.initial[pid] = bool(ctrl.get("active"))
                elif shape == BUTTON:
                    self.kind[pid] = "button"
                elif "luminance" in ctrl:
                    self.kind[pid] = "lamp"
                else:
                    self.kind[pid] = "external"
                for target in ctrl.get("controllers") or []:
                    self.inputs.setdefault(target["id"], []).append(pid)
        # wires to parts that are not in the blueprint's logic are ignored
        self.inputs = {k: v for k, v in self.inputs.items() if k in self.kind}
        self.state = {pid: self.initial.get(pid, False) for pid in self.kind}
        self.forced = {}
        self.history = {pid: [] for pid, k in self.kind.items() if k == "timer"}
        self.tick_count = 0

    def force(self, pid, value):
        """Drive a part from outside, like a switch."""
        self.forced[pid] = bool(value)

    def release(self, pid):
        self.forced.pop(pid, None)

    def get(self, pid):
        return self.state[pid]

    def tick(self, n=1):
        for _ in range(n):
            prev, new = self.state, {}
            for pid, kind in self.kind.items():
                if pid in self.forced:
                    new[pid] = self.forced[pid]
                    continue
                ins = [prev[i] for i in self.inputs.get(pid, ())]
                if kind == "gate":
                    new[pid] = gate(self.mode[pid], ins)
                elif kind == "timer":
                    hist = self.history[pid]
                    hist.append(any(ins))
                    d = self.delay[pid]
                    new[pid] = hist[-1 - d] if len(hist) > d else False
                    if len(hist) > d + 1:
                        hist.pop(0)
                elif kind == "lamp":
                    new[pid] = any(ins)
                else:
                    new[pid] = prev[pid]
            self.state = new
            self.tick_count += 1

    def settle(self, max_ticks=1000):
        """Tick until nothing changes. Returns ticks used, or -1 if it never settles (a clock)."""
        # a timer can hide a change for its whole delay, so require that many quiet ticks
        quiet_needed = max(self.delay.values(), default=0) + 2
        quiet = 0
        for t in range(max_ticks):
            before = self.state
            self.tick()
            quiet = quiet + 1 if self.state == before else 0
            if quiet >= quiet_needed:
                return t + 1 - quiet_needed
        return -1


def gate(mode, ins):
    if not ins:
        return False
    on = sum(ins)
    if mode in (AND, NAND):
        r = on == len(ins)
    elif mode in (OR, NOR):
        r = on > 0
    else:
        r = on % 2 == 1
    return (not r) if mode >= 3 else r


def write_bits(sim, pids, value):
    for i, p in enumerate(pids):
        sim.force(p, (value >> i) & 1)


def read_bits(sim, pids):
    return sum(int(sim.get(p)) << i for i, p in enumerate(pids))
