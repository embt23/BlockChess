"""Bits per ply, for every encoding on the table.

Nothing here needs a chess engine, because none of these costs depend on which
moves are legal -- only on how many are, and on the shape of the distribution
over them. The one input this file does not have is the empirical branching
histogram of a real corpus; where that is needed the answer is given as a
function of b rather than as a number. See EXPERIMENTS.md.
"""

from math import ceil, log2

GAME_PLIES = 80          # a 40-move game, the usual round number
MAX_LEGAL_MOVES = 218    # the most legal moves any position is known to have


def per_ply_constants():
    """Schemes whose cost does not depend on the position."""
    return [
        ("SAN text, as in PGN",              8 * 5.0),
        ("UCI text, 'e2e4' + separator",     8 * 5.0),
        ("from|to|promo|flag, 16 bits",      16.0),
        ("from|to, 12 bits + escape",        12.0),
        ("legal-move index, one byte",       8.0),
        ("legal-move index, worst case",     log2(MAX_LEGAL_MOVES)),
    ]


def index_cost(b):
    """Cost of naming one of b legal moves: rounded up to whole bits, and
    exactly, as an arithmetic coder would spend it."""
    return ceil(log2(b)), log2(b)


def zipf(k, s=1.0):
    w = [1.0 / (i ** s) for i in range(1, k + 1)]
    t = sum(w)
    return [x / t for x in w]


def entropy(p):
    return -sum(x * log2(x) for x in p if x > 0)


def trie_vs_model(k=4096, s=1.0):
    """A book of k opening lines with a Zipf popularity distribution.

    Two ways to say 'this game follows line i':
      - a pointer, which costs log2(k) bits whichever line it is;
      - an entropy code, which costs -log2(p_i) bits, on average H(p).

    They are the same mechanism; the pointer is the entropy code you get when
    you refuse to use the popularity information you already have."""
    p = zipf(k, s)
    return log2(k), entropy(p)


if __name__ == "__main__":
    print(f"{'scheme':<36} {'bits/ply':>9} {'bytes/game':>11}")
    for name, bits in per_ply_constants():
        print(f"{name:<36} {bits:>9.2f} {bits * GAME_PLIES / 8:>11.1f}")
    print()

    print("legal-move index, as a function of the branching factor b")
    print(f"{'b':>5} {'ceil bits':>10} {'exact bits':>11} {'bytes/game':>11}")
    for b in (1, 5, 10, 20, 25, 30, 35, 40, 60, MAX_LEGAL_MOVES):
        c, e = index_cost(b)
        print(f"{b:>5} {c:>10} {e:>11.3f} {e * GAME_PLIES / 8:>11.1f}")
    print()
    print("  E[log2 b] over a real corpus is the number that matters and it is")
    print("  strictly below log2 E[b] by Jensen. Measuring it is experiment X1.")
    print()

    print("storing a game as positions instead of moves")
    for label, bits in (("naive 13^64 packing", 236.83),
                        ("derived bound B6 (counting.py)", 172.05),
                        ("literature estimate of legal positions", 148.4)):
        print(f"  {label:<40} {bits:>7.1f} bits x {GAME_PLIES} plies"
              f" = {bits * GAME_PLIES / 8 / 1024:>6.2f} KiB/game")
    print()

    print("pointer into an opening book vs entropy coding the same choice")
    for k in (256, 4096, 65536):
        ptr, h = trie_vs_model(k)
        print(f"  {k:>6} lines, Zipf s=1 : pointer {ptr:>5.1f} bits,"
              f" entropy {h:>5.1f} bits, waste {ptr - h:>4.1f}")
