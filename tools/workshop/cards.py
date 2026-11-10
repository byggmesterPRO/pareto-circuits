"""Make every build ready to post on the Steam Workshop.

    python tools/workshop/cards.py            every build slot
    python tools/workshop/cards.py adder      one family
    python tools/workshop/cards.py --balanced the balanced builds too (left out for now: only fast and small
                                              go on the Workshop)

For each slot under circuits/<family>/builds/ this writes, from meta.json and blueprint.json:

- blueprint/description.json: the description the game shows and uploads (the name and localId stay as they are)
- blueprint/icon.png: the front image, with what the build is, how it works, its numbers, and its own layout

Run it after `pareto build ... --write`. Needs Pillow.
"""
import json
import re
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
REPO = "github.com/byggmesterPRO/pareto-circuits"
PROFILES = {"fast": "fewest ticks", "small": "fewest gates", "balanced": "lowest gates x ticks"}


def x(a, b=None):
    """Where pins sit in a row: "x 0-7" or "x 8"."""
    return f"x {a}" if b is None or b == a else f"x {a}-{b}"


def log2(n):
    return n.bit_length() - 1


def two(what, flag=None):
    """Two n-bit numbers in, a result (and maybe a flag) out."""
    def make(n):
        outs = f"result = {x(0, n - 1)}" + (f", {flag} = {x(n)}" if flag else "")
        return what.format(n=n), f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}", outs
    return make


def one(what, outs):
    return lambda n: (what.format(n=n), f"the number = {x(0, n - 1)}", outs(n))


# family: (title, size -> (what it does, inputs, outputs))
FAMILIES = {
    "adder": ("Adder", two("Adds two {n}-bit numbers.", "carry")),
    "subtractor": ("Subtractor", two("Takes b from a, two {n}-bit numbers.", "borrow (a < b)")),
    "adder-subtractor": ("Adder-Subtractor", lambda n: (
        f"Adds two {n}-bit numbers, or subtracts them when the mode pin is on.",
        f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}, mode = {x(2 * n)} (on subtracts)",
        f"result = {x(0, n - 1)}, carry or borrow = {x(n)}")),
    "multiplier": ("Multiplier", lambda n: (
        f"Multiplies two {n}-bit numbers.", f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}", f"product = {x(0, 2 * n - 1)}")),
    "divider": ("Divider", lambda n: (
        f"Divides a by b, two {n}-bit numbers. Dividing by 0 gives all ones.",
        f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}", f"quotient = {x(0, n - 1)}, remainder = {x(n, 2 * n - 1)}")),
    "comparator": ("Comparator", lambda n: (
        f"Compares two {n}-bit numbers.", f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}", "a < b = x 0, a = b = x 1, a > b = x 2")),
    "incrementer": ("Incrementer", one("Adds 1 to a {n}-bit number.", lambda n: f"result = {x(0, n - 1)}, carry = {x(n)}")),
    "decrementer": ("Decrementer", one("Takes 1 from a {n}-bit number.", lambda n: f"result = {x(0, n - 1)}, borrow = {x(n)}")),
    "negate": ("Negate", one("Turns a {n}-bit number into its negative (two's complement).", lambda n: f"result = {x(0, n - 1)}")),
    "absolute": ("Absolute Value", one("Drops the sign of a {n}-bit two's complement number.", lambda n: f"result = {x(0, n - 1)}")),
    "square": ("Square", one("Multiplies a {n}-bit number by itself.", lambda n: f"square = {x(0, 2 * n - 1)}")),
    "square-root": ("Square Root", one("The square root of a {n}-bit number, rounded down.", lambda n: f"root = {x(0, n // 2 - 1)}")),
    "alu": ("ALU", lambda n: (
        f"Eight operations on two {n}-bit numbers: 0 add, 1 subtract, 2 and, 3 or, 4 xor, 5 shift up, 6 shift down, 7 not.",
        f"a = {x(0, n - 1)}, b = {x(n, 2 * n - 1)}, operation = {x(2 * n, 2 * n + 2)}",
        f"result = {x(0, n - 1)}, flag (carry, borrow or the bit shifted out) = {x(n)}")),
    "double-dabble": ("Double Dabble", lambda n: (
        f"Turns a {n}-bit binary number into decimal digits (BCD).",
        f"the number = {x(0, n - 1)}", f"{len(str(2 ** n - 1))} digits of 4 bits, ones digit first, from x 0")),
    "decoder": ("Decoder", lambda n: (
        f"{n} bits in, {2 ** n} lines out: only the line of the current value is on.",
        f"the value = {x(0, n - 1)}",
        f"line k = x k" if n < 4 else f"a square of {2 ** (n // 2)} per row, line 0 first, behind the inputs")),
    "demultiplexer": ("Demultiplexer", lambda n: (
        f"Sends one data input to one of {n} lines.",
        f"data = x 0, line number = {x(1, log2(n))}",
        "line k = x k" if n < 4 else f"a matrix of {2 ** (log2(n) // 2)} per row, line 0 first, behind the inputs")),
    "multiplexer": ("Multiplexer", lambda n: (
        f"Sends one of {n} data lines to the output.",
        f"data lines = {x(0, n - 1)}, line number = {x(n, n + log2(n) - 1)}", "the chosen line = x 0")),
    "barrel-shifter": ("Barrel Shifter", lambda n: (
        f"Shifts a {n}-bit number up by 0 to {n - 1} places.",
        f"the number = {x(0, n - 1)}, places = {x(n, n + log2(n) - 1)}", f"result = {x(0, n - 1)}")),
    "seven-segment": ("Seven Segment", lambda n: (
        "Turns a hex digit (0-9, A-F) into the seven segments of a display.",
        "the digit = x 0-3", "segments a to g = x 0-6 (a top, then clockwise, g middle)")),
    "register": ("Register", lambda n: (
        f"Keeps a {n}-bit number until the next write.",
        f"data = {x(0, n - 1)}, write = {x(n)} (a pulse of exactly one tick)", f"the stored number = {x(0, n - 1)}")),
    "counter": ("Counter", lambda n: (
        f"Counts pulses, 0 to {2 ** n - 1} and around again.",
        "count = x 0, reset = x 1 (pulses of exactly one tick)", f"the count = {x(0, n - 1)}")),
}


def ram(size):
    words, bits = (int(v) for v in re.findall(r"\d+", size))
    a = log2(words)
    return (f"{words} words of {bits} bits. The word at the address is always on the outputs.",
            f"address = {x(0, a - 1)}, data = {x(a, a + bits - 1)}, write = {x(a + bits)} (a pulse of exactly one tick)",
            f"the word at the address = {x(0, bits - 1)}")


# how a design works, by the start of its name
HOW = [
    ("ripple", "One small cell per bit; the carry walks from bit to bit."),
    ("lookahead", "Every bit sees its carry in one step, so the ticks do not grow with the size."),
    ("blocks", "Lookahead inside blocks of bits; the carry walks from block to block."),
    ("two-level", "Lookahead inside blocks, and a second lookahead across the blocks."),
    ("tree", "Full adders in layers bring the columns down to two rows, then a fast adder."),
    ("wide", "Counters of up to 7 bits bring the columns down in few layers, then a fast adder."),
    ("normal-form", "Every output is one XOR gate over AND gates of plain inputs: 3 ticks."),
    ("no-inverters", "One XOR gate over AND gates of plain inputs, so no inverter tick."),
    ("flat-nor", "One AND gate per bit position; the third output is 'neither of the others'."),
    ("flat", "One wide gate per term: gates take any number of inputs, so no stages are needed."),
    ("stages", "The amount is taken a few bits at a time, one stage per group."),
    ("table", "One line per value, and each output is an OR over lines."),
    ("searched", "Found by computer search for the fewest gates."),
    ("chain", "A chain of small add-3 cells, each found by exact search."),
    ("toggle", "Each bit is an XOR gate wired to itself; a pulse flips the bits that must change."),
    ("direct", "Each bit is an XOR gate wired to itself; every cell reads the address itself."),
    ("lines", "Each bit is an XOR gate wired to itself; every word has its own select line."),
    ("matrix", "Row and column lines, and each output is the AND of its row and column."),
]

COLORS = {"E8C547": (232, 197, 71), "19E753": (25, 231, 83)}
BG, PANEL, TEXT, DIM, GATE = (18, 20, 24), (30, 33, 40), (236, 238, 242), (150, 156, 168), (74, 80, 94)
YELLOW, GREEN = COLORS["E8C547"], COLORS["19E753"]
FONTS = Path("C:/Windows/Fonts")


def font(size, bold=False):
    for name in (["bahnschrift.ttf"] if bold else []) + ["segoeui.ttf", "arial.ttf"]:
        try:
            return ImageFont.truetype(str(FONTS / name), size)
        except OSError:
            pass
    return ImageFont.load_default()


def wrap(draw, text, fnt, width):
    lines, line = [], ""
    for word in text.split():
        trial = (line + " " + word).strip()
        if draw.textlength(trial, font=fnt) <= width or not line:
            line = trial
        else:
            lines.append(line)
            line = word
    return lines + [line]


def describe(meta):
    fam, size = meta["family"], meta["size"]
    title, make = FAMILIES.get(fam, ("RAM", None))
    what, ins, outs = ram(size) if fam == "ram" else make(int(size.removesuffix("bit")))
    design = re.match(r"Generator design: ([^.]*)\.", meta.get("notes", ""))
    design = design.group(1) if design else meta.get("source", "search")
    how = next((text for start, text in HOW if design.startswith(start)), "")
    return title, what, ins, outs, design, how


def checked(meta):
    if meta["family"] in ("ram", "register", "counter"):
        return "Checked with thousands of random operations in a simulator."
    if meta["verified"] == "exhaustive":
        return "Checked on every possible input in a simulator."
    return "Checked on about a million inputs in a simulator."


def about(fam):
    """The family's own words for the Workshop (circuits/<family>/workshop.md): what it is for and how to use it."""
    path = ROOT / "circuits" / fam / "workshop.md"
    if not path.exists():
        return []
    return [path.read_text(encoding="utf-8").strip(), ""]


def description(meta):
    title, what, ins, outs, design, how = describe(meta)
    timing = re.search(r"Ticks from an input.*", meta.get("notes", ""))
    lines = [
        f"v{meta['version']} | {meta['gates']} gates | {meta['ticks']} ticks",
        f"https://{REPO}",
        "",
        what,
        "",
        *about(meta["family"]),
        f"Inputs (yellow gates, first row, lowest bit first): {ins}.",
        f"Outputs (green gates): {outs}.",
        "",
        f"Profile: {meta['profile']}, the build with the {PROFILES[meta['profile']]} for this size.",
        f"How it works: {how}" if how else f"Design: {design}.",
    ]
    if timing:
        lines.append(timing.group(0))
    lines += [
        f"{checked(meta)} Gates include the input gates; ticks count the input gate too.",
        "",
        "Other sizes, the generator and the full explanation:",
        f"https://{REPO}/tree/main/circuits/{meta['family']}",
    ]
    return "\n".join(lines)


def layout(slot, box):
    """The build seen from above, scaled to fit the box: yellow inputs, green outputs, grey gates."""
    bp = json.loads((slot / "blueprint" / "blueprint.json").read_text())
    cells = {}
    for body in bp["bodies"]:
        for c in body["childs"]:
            cells[(c["pos"]["x"], c["pos"]["y"])] = COLORS.get(c.get("color", "").upper(), GATE)
    xs, ys = [p[0] for p in cells], [p[1] for p in cells]
    w, h = max(xs) - min(xs) + 1, max(ys) - min(ys) + 1
    small = Image.new("RGB", (w, h), PANEL)
    for (cx, cy), color in cells.items():
        small.putpixel((cx - min(xs), cy - min(ys)), color)
    if h > w:           # a deep, narrow build is shown on its side, inputs to the left
        small = small.transpose(Image.TRANSPOSE)
        w, h = h, w
    scale = min(box[0] / w, box[1] / h, 26)
    size = (max(1, round(w * scale)), max(1, round(h * scale)))
    big = small.resize(size, Image.NEAREST if scale >= 1 else Image.BOX)
    if scale >= 5:      # thin lines between the gates
        d = ImageDraw.Draw(big)
        for i in range(1, w):
            d.line([(round(i * scale), 0), (round(i * scale), size[1])], fill=PANEL)
        for j in range(1, h):
            d.line([(0, round(j * scale)), (size[0], round(j * scale))], fill=PANEL)
    return big


def icon(slot, meta):
    title, what, ins, outs, design, how = describe(meta)
    S = 512
    img = Image.new("RGB", (S, S), BG)
    d = ImageDraw.Draw(img)
    pad = 28
    # title and size
    d.text((pad, 22), title.upper(), font=font(46, True), fill=TEXT)
    tag = f"{meta['size']}  {meta['profile'].upper()}"
    d.text((pad, 78), tag, font=font(26, True), fill=YELLOW)
    y = 122
    for line in wrap(d, what, font(19), S - 2 * pad)[:3]:
        d.text((pad, y), line, font=font(19), fill=TEXT)
        y += 25
    # the layout of this build
    top, bottom = y + 10, 322
    d.rectangle([pad, top, S - pad, bottom], fill=PANEL)
    pic = layout(slot, (S - 2 * pad - 20, bottom - top - 20))
    img.paste(pic, ((S - pic.width) // 2, top + (bottom - top - pic.height) // 2))
    # numbers
    y = 336
    third = (S - 2 * pad) // 3
    numbers = [(f"{meta['gates']:,}".replace(",", " "), "GATES"), (str(meta["ticks"]), "TICKS"), (f"v{meta['version']}", meta["verified"].upper() if meta["family"] not in ("ram", "register", "counter") else "SIMULATED")]
    for k, (value, label) in enumerate(numbers):
        cx = pad + k * third
        d.text((cx, y), value, font=font(40, True), fill=GREEN if k < 2 else TEXT)
        d.text((cx + 2, y + 46), label, font=font(15), fill=DIM)
    # how it works
    y = 410
    for line in wrap(d, how or f"Design: {design}", font(17), S - 2 * pad)[:2]:
        d.text((pad, y), line, font=font(17), fill=DIM)
        y += 22
    d.line([(pad, 466), (S - pad, 466)], fill=PANEL, width=2)
    d.text((pad, 476), REPO, font=font(17, True), fill=TEXT)
    img.save(slot / "blueprint" / "icon.png", optimize=True)


def main():
    only = [a for a in sys.argv[1:] if not a.startswith("--")]
    profiles = ("fast", "small", "balanced") if "--balanced" in sys.argv else ("fast", "small")
    done = 0
    for meta_path in sorted((ROOT / "circuits").glob("*/builds/*/*/meta.json")):
        meta = json.loads(meta_path.read_text())
        if (only and meta["family"] not in only) or meta["profile"] not in profiles:
            continue
        slot = meta_path.parent
        desc_path = slot / "blueprint" / "description.json"
        desc = json.loads(desc_path.read_text())
        desc["description"] = description(meta)
        desc_path.write_text(json.dumps(desc, indent=2) + "\n", newline="\n")
        icon(slot, meta)
        done += 1
    print(done, "builds")


if __name__ == "__main__":
    main()
