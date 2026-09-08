"""How big is the space of chess positions?

A ladder of upper bounds, each one derived from the previous by adding a single
constraint, so the cost of each constraint is visible in bits. Every number
here is computed, not quoted. Literature values appear only in the papers, and
only as a comparison at the end of the ladder.
"""

from math import comb, log2, factorial
from itertools import product

from board import N, king_neighbours, rank, PAWN_LEGAL, BACK_RANKS

# --------------------------------------------------------------------------
# Legal king placements. The only rule of chess that is a pure two-body
# constraint, and the cleanest place to see the board's border in a number.
# --------------------------------------------------------------------------


def king_pairs():
    """Ordered (white king, black king) pairs that are distinct and
    non-adjacent, bucketed by how many of the two sit on a back rank."""
    buckets = {0: 0, 1: 0, 2: 0}
    for wk in range(N):
        nb = set(king_neighbours(wk))
        for bk in range(N):
            if bk == wk or bk in nb:
                continue
            kb = (rank(wk) in (0, 7)) + (rank(bk) in (0, 7))
            buckets[kb] += 1
    return buckets


# --------------------------------------------------------------------------
# Material. Per side: p pawns and (n, b, r, q) other pieces. Two constraints
# are enforced:
#
#   total     p + n + b + r + q <= 15          (16 men including the king)
#   promotion max(0,n-2) + max(0,b-2) + max(0,r-2) + max(0,q-1) <= 8 - p
#
# The promotion constraint is the interesting one: every piece beyond the
# starting complement had to arrive by promoting a pawn, and every promotion
# spends a pawn. Not enforced: bishops needing opposite colour complexes,
# pawn-structure reachability, whose turn it is being consistent with who is
# in check. Those are noted in the paper as the residual gap.
# --------------------------------------------------------------------------

MAX_OF = {"n": 10, "b": 10, "r": 10, "q": 9}


def side_egf(p: int) -> list[float]:
    """Exponential generating function for one side's non-pawn, non-king men,
    given it has p pawns. Returns coefficients c[m] = sum over valid material
    vectors of total size m of 1/(n! b! r! q!)."""
    budget = 8 - p
    coef = [0.0] * 16
    for n in range(MAX_OF["n"] + 1):
        if max(0, n - 2) > budget:
            break
        for b in range(MAX_OF["b"] + 1):
            if max(0, n - 2) + max(0, b - 2) > budget:
                break
            for r in range(MAX_OF["r"] + 1):
                if max(0, n - 2) + max(0, b - 2) + max(0, r - 2) > budget:
                    break
                for q in range(MAX_OF["q"] + 1):
                    used = (max(0, n - 2) + max(0, b - 2)
                            + max(0, r - 2) + max(0, q - 1))
                    if used > budget:
                        break
                    m = n + b + r + q
                    if p + m > 15:
                        break
                    coef[m] += 1.0 / (factorial(n) * factorial(b)
                                      * factorial(r) * factorial(q))
    return coef


def placements_with_material_caps() -> int:
    """Count piece placements subject to: two non-adjacent kings, pawns only on
    ranks 2-7, and the material caps above. Exact under those constraints."""
    egf = {p: side_egf(p) for p in range(9)}
    total = 0
    for kb, npairs in king_pairs().items():
        kp = 2 - kb                       # kings standing on pawn-legal squares
        pawn_free = len(PAWN_LEGAL) - kp  # pawn-legal squares left empty
        for wp in range(9):
            wways = comb(pawn_free, wp)
            if wways == 0:
                continue
            for bp in range(9):
                bways = comb(pawn_free - wp, bp)
                if bways == 0:
                    continue
                free = (N - 2) - wp - bp  # squares left for the other men
                conv = [0.0] * 31
                a, b_ = egf[wp], egf[bp]
                for i, ai in enumerate(a):
                    if ai == 0.0:
                        continue
                    for j, bj in enumerate(b_):
                        if bj == 0.0:
                            continue
                        conv[i + j] += ai * bj
                inner = sum(comb(free, m) * factorial(m) * c
                            for m, c in enumerate(conv) if c and m <= free)
                total += npairs * wways * bways * inner
    return total


def ladder():
    """The bounds, in order, with the bit cost of each."""
    kp = king_pairs()
    npairs = sum(kp.values())

    rows = []
    rows.append(("B0  every square holds one of 13 states",
                 13.0 ** 64))
    rows.append(("B1  + pawns only on ranks 2-7",
                 13.0 ** len(PAWN_LEGAL) * 11.0 ** len(BACK_RANKS)))

    # B2: exactly one king per side, non-adjacent; every other square is one of
    # 11 states (empty + 5 white + 5 black non-king types), or 9 on a back rank.
    b2 = 0.0
    for kb, cnt in kp.items():
        back_free = len(BACK_RANKS) - kb
        other_free = (N - 2) - back_free
        b2 += cnt * (9.0 ** back_free) * (11.0 ** other_free)
    rows.append(("B2  + exactly two kings, non-adjacent", b2))

    b3 = float(placements_with_material_caps())
    rows.append(("B3  + <=16 men a side, promotion budget", b3))

    # State beyond the placement: side to move, castling rights, en passant
    # file. Upper bound only -- most of these combinations are not reachable.
    rows.append(("B4  x side to move (2)", b3 * 2))
    rows.append(("B5  x castling rights (<=16)", b3 * 2 * 16))
    rows.append(("B6  x en passant file (<=9)", b3 * 2 * 16 * 9))
    return npairs, rows


if __name__ == "__main__":
    kp = king_pairs()
    print(f"king pairs (ordered, non-adjacent) = {sum(kp.values())}")
    print(f"  by kings on back ranks: {kp}")
    print()
    _, rows = ladder()
    prev = None
    print(f"{'bound':<44} {'count':>12} {'bits':>8} {'saved':>7}")
    for name, v in rows:
        bits = log2(v)
        saved = "" if prev is None else f"{prev - bits:+.1f}"
        print(f"{name:<44} {v:>12.3e} {bits:>8.2f} {saved:>7}")
        prev = bits
