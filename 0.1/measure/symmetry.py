"""What symmetry group actually acts on a chess position, and what it is worth.

The square has the dihedral group D4 of order 8. Chess does not: pawns pick out
a rank direction, castling picks out a file direction, and the board has edges
rather than wrapping. What survives is measured here, together with the number
of bits that surviving symmetry could ever save.

A group element is (square permutation, swap colours). Swapping colours also
flips the side to move, so the operation maps a position to one that is the
same position from the other player's point of view.
"""

from math import log2

from board import N, D4, king_neighbours, rank, PAWN_LEGAL

W, B = 0, 1

# Pawnless, no castling rights, no en passant: everything commutes with the
# rules, so the whole of D4 acts, and so does the colour swap.
G_PAWNLESS = [(p, s) for p in D4 for s in (False, True)]

# With pawns on the board: a rank-reversing permutation makes pawns move
# backwards, so it is only legal when paired with a colour swap. The file
# mirror is fine on its own.
G_PAWNS = [("e", False), ("m_file", False), ("m_rank", True), ("rot180", True)]

# With castling rights outstanding the file mirror sends the king from e1 to
# d1, which is not a chess position. Only the colour swap survives.
G_CASTLING = [("e", False), ("m_rank", True)]


def act(pos, elem):
    """pos = (frozenset of (colour, piece, square), side_to_move)."""
    perm_name, swap = elem
    perm = D4[perm_name]
    men, stm = pos
    out = frozenset(((c ^ 1 if swap else c), p, perm[sq]) for c, p, sq in men)
    return (out, stm ^ 1 if swap else stm)


def orbit_stats(positions, group):
    """Burnside by brute force: canonicalise each position and count distinct
    representatives. Returns (|X|, number of orbits, bits saved)."""
    canon = set()
    for pos in positions:
        images = [act(pos, g) for g in group]
        canon.add(min((tuple(sorted(m)), s) for m, s in images))
    size, orbits = len(positions), len(canon)
    return size, orbits, log2(size / orbits)


def legal_king_pairs():
    for wk in range(N):
        nb = set(king_neighbours(wk))
        for bk in range(N):
            if bk != wk and bk not in nb:
                yield wk, bk


def family_kk():
    for wk, bk in legal_king_pairs():
        for stm in (0, 1):
            yield (frozenset({(W, "K", wk), (B, "K", bk)}), stm)


def family_k_plus(piece, squares=None, colours=(W,)):
    """Kings plus one extra man. `squares` restricts where it may stand.
    `colours` decides whether the family is closed under the colour swap --
    it is not, if only one side may own the extra man, and then the colour
    swap contributes nothing."""
    allowed = range(N) if squares is None else squares
    for wk, bk in legal_king_pairs():
        for sq in allowed:
            if sq in (wk, bk):
                continue
            for colour in colours:
                for stm in (0, 1):
                    yield (frozenset({(W, "K", wk), (B, "K", bk),
                                      (colour, piece, sq)}), stm)


CASES = [
    ("K vs K            pawnless", family_kk, G_PAWNLESS),
    ("KQ vs K           pawnless", lambda: family_k_plus("Q"), G_PAWNLESS),
    ("KQ vs K / K vs KQ pawnless",
     lambda: family_k_plus("Q", colours=(W, B)), G_PAWNLESS),
    ("KP vs K           a pawn  ",
     lambda: family_k_plus("P", PAWN_LEGAL), G_PAWNS),
    ("KP vs K / K vs KP a pawn  ",
     lambda: family_k_plus("P", PAWN_LEGAL, colours=(W, B)), G_PAWNS),
    ("KP vs K           castling",
     lambda: family_k_plus("P", PAWN_LEGAL), G_CASTLING),
]

if __name__ == "__main__":
    print("group orders")
    print(f"  no pawns, no castling : {len(G_PAWNLESS):>3}   (D4 x colour swap)")
    print(f"  pawns on the board    : {len(G_PAWNS):>3}   (file mirror x colour swap)")
    print(f"  castling rights alive : {len(G_CASTLING):>3}   (colour swap only)")
    print(f"  general position      : {1:>3}   (nothing survives)")
    print()
    print(f"{'family':<32} {'|X|':>10} {'orbits':>10} {'bits saved':>11} {'ceiling':>8}")
    for name, gen, group in CASES:
        size, orbits, bits = orbit_stats(list(gen()), group)
        print(f"{name:<32} {size:>10} {orbits:>10} {bits:>11.3f} {log2(len(group)):>8.2f}")
