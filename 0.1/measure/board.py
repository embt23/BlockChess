"""Shared board geometry. Squares are 0..63 with sq = 8*rank + file, a1 = 0."""

N = 64
RANKS = 8
FILES = 8


def rank(sq: int) -> int:
    return sq // 8


def file(sq: int) -> int:
    return sq % 8


def king_neighbours(sq: int) -> list[int]:
    """The squares a king on `sq` attacks. This is where the border shows up:
    the count is 3, 5 or 8 depending on how much of the 3x3 neighbourhood
    falls off the edge of the board."""
    r, f = rank(sq), file(sq)
    out = []
    for dr in (-1, 0, 1):
        for df in (-1, 0, 1):
            if dr == 0 and df == 0:
                continue
            nr, nf = r + dr, f + df
            if 0 <= nr < RANKS and 0 <= nf < FILES:
                out.append(8 * nr + nf)
    return out


# The eight elements of D4, as permutations of the 64 squares, written as
# maps on (rank, file). `id` first; the rest are named by what they do.
def _perm(fn):
    return tuple(8 * fn(rank(s), file(s))[0] + fn(rank(s), file(s))[1] for s in range(N))


D4 = {
    "e": _perm(lambda r, f: (r, f)),                    # identity
    "m_file": _perm(lambda r, f: (r, 7 - f)),           # mirror a<->h
    "m_rank": _perm(lambda r, f: (7 - r, f)),           # mirror rank 1<->8
    "rot180": _perm(lambda r, f: (7 - r, 7 - f)),
    "d_main": _perm(lambda r, f: (f, r)),               # reflect in a1-h8
    "d_anti": _perm(lambda r, f: (7 - f, 7 - r)),       # reflect in a8-h1
    "rot90": _perm(lambda r, f: (f, 7 - r)),
    "rot270": _perm(lambda r, f: (7 - f, r)),
}

PAWN_LEGAL = [s for s in range(N) if 1 <= rank(s) <= 6]   # ranks 2..7
BACK_RANKS = [s for s in range(N) if rank(s) in (0, 7)]   # ranks 1 and 8
