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


FAMILIES = {"double-dabble": double_dabble}
