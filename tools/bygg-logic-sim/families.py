"""What each circuit family has to compute. The verifier checks builds against these.

A family is a function size -> (input bits, output bits, f) where f(x) is the correct output for input x.
Size is the folder name, for example "12bit".
"""


def double_dabble(size):
    """Binary to BCD. Output digit d (ones first) is bits 4d .. 4d+3, least significant first."""
    bits = int(size.removesuffix("bit"))
    digits = len(str((1 << bits) - 1))

    def f(x):
        out = 0
        for d in range(digits):
            out |= (x // 10 ** d % 10) << (4 * d)
        return out

    return bits, 4 * digits, f


def seven_segment(size):
    """One hexadecimal digit to segments a .. g (bits 0 .. 6)."""
    if size != "4bit":
        raise ValueError("seven-segment: the only size is 4bit")
    glyphs = [0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F, 0x77, 0x7C, 0x39, 0x5E, 0x79, 0x71]
    return 4, 7, lambda x: glyphs[x]


FAMILIES = {"double-dabble": double_dabble, "seven-segment": seven_segment}
