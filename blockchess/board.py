"""Geometry and scope for partially-recorded positions.

BlockChess positions are *fragments*: a handful of men written down from a game
in progress, often without the kings, often not in order of placement. That is a
different object from a chess position, and this module is built for it.

A fragment is stored as a sparse map ``{"a3": "wB", ...}`` plus a roster of men
known to exist but not written down. Move generation is pseudo-legal, which is
exact when no king is recorded (nothing can be in check) and is filtered for
check only when both kings are present.
"""

from __future__ import annotations

FILES = "abcdefgh"
ORTHO = ((1, 0), (-1, 0), (0, 1), (0, -1))
DIAG = ((1, 1), (1, -1), (-1, 1), (-1, -1))
KNIGHT = ((1, 2), (2, 1), (2, -1), (1, -2), (-1, -2), (-2, -1), (-2, 1), (-1, 2))
SLIDE = {"B": DIAG, "R": ORTHO, "Q": ORTHO + DIAG}
KIND_NAME = {"P": "pawn", "N": "knight", "B": "bishop",
             "R": "rook", "Q": "queen", "K": "king"}


def coords(square: str) -> tuple[int, int]:
    return FILES.index(square[0]), int(square[1:])


def name(file_idx: int, rank: int) -> str:
    return FILES[file_idx] + str(rank)


def on_board(file_idx: int, rank: int) -> bool:
    return 0 <= file_idx < 8 and 1 <= rank <= 8


def square_color(square: str) -> str:
    """Dark or light. Kept separate from piece colour on purpose: the colour
    complex is a property of the board, the side to move is not."""
    fi, rank = coords(square)
    return "dark" if (fi + 1 + rank) % 2 == 0 else "light"


ALL_SQUARES = [name(f, r) for r in range(8, 0, -1) for f in range(8)]


class Ray:
    """One direction out of a square, up to and including whatever stops it.

    ``open_squares`` are the empty squares reached. ``blocker`` is the square
    that ended the ray (None if it ran off the board), and ``blocked_by_own``
    says whether the piece was stopped by its own side -- the distinction that
    separates a bishop with somewhere to go from a bishop staring at its own
    pawn.
    """

    __slots__ = ("origin", "direction", "open_squares", "blocker",
                 "blocker_man", "blocked_by_own")

    def __init__(self, origin, direction, open_squares, blocker,
                 blocker_man, blocked_by_own):
        self.origin = origin
        self.direction = direction
        self.open_squares = open_squares
        self.blocker = blocker
        self.blocker_man = blocker_man
        self.blocked_by_own = blocked_by_own

    @property
    def reach(self) -> list[str]:
        """Squares the piece attacks along this ray, blocker included."""
        return self.open_squares + ([self.blocker] if self.blocker else [])

    def __repr__(self):
        tail = f"->{self.blocker}({self.blocker_man})" if self.blocker else "->edge"
        return f"<Ray {self.origin}{self.direction} {self.open_squares}{tail}>"


class Board:
    """A recorded fragment.

    ``men`` maps square -> two-character man, e.g. ``"wB"``.
    ``unrecorded`` lists men known to be on the board but not written down,
    e.g. ``["wK", "bK"]``. It carries no position -- that is the point.
    """

    def __init__(self, men=None, side_to_move="w", unrecorded=None, label=None):
        self.men = dict(men or {})
        self.side_to_move = side_to_move
        self.unrecorded = list(unrecorded or [])
        self.label = label
        for square in self.men:
            if square not in ALL_SQUARES:
                raise ValueError(f"not a square: {square!r}")

    # -- basics ---------------------------------------------------------
    def copy(self) -> "Board":
        return Board(self.men, self.side_to_move, self.unrecorded, self.label)

    def at(self, square):
        return self.men.get(square)

    def color_at(self, square):
        man = self.men.get(square)
        return man[0] if man else None

    def squares_of(self, color=None, kind=None):
        return [sq for sq, man in self.men.items()
                if (color is None or man[0] == color)
                and (kind is None or man[1] == kind)]

    @property
    def empty_squares(self):
        return [sq for sq in ALL_SQUARES if sq not in self.men]

    def has_both_kings(self):
        return bool(self.squares_of("w", "K")) and bool(self.squares_of("b", "K"))

    # -- rays -----------------------------------------------------------
    def ray(self, square, direction) -> Ray:
        fi, rank = coords(square)
        mover = self.men[square]
        df, dr = direction
        open_squares = []
        fi, rank = fi + df, rank + dr
        while on_board(fi, rank):
            here = name(fi, rank)
            man = self.men.get(here)
            if man:
                return Ray(square, direction, open_squares, here, man,
                           man[0] == mover[0])
            open_squares.append(here)
            fi, rank = fi + df, rank + dr
        return Ray(square, direction, open_squares, None, None, False)

    def rays(self, square) -> list[Ray]:
        """Every ray of a sliding piece. Empty list for non-sliders."""
        man = self.men.get(square)
        if not man or man[1] not in SLIDE:
            return []
        return [self.ray(square, d) for d in SLIDE[man[1]]]

    # -- attacks and moves ----------------------------------------------
    def attacks(self, square) -> set[str]:
        """Squares this man bears on, including squares holding friendly men.

        Attacking a friendly man is how "defends" is expressed, so this is the
        set to use for control questions; use ``destinations`` for mobility.
        """
        man = self.men.get(square)
        if not man:
            return set()
        color, kind = man[0], man[1]
        fi, rank = coords(square)
        out = set()
        if kind == "P":
            step = 1 if color == "w" else -1
            for df in (-1, 1):
                if on_board(fi + df, rank + step):
                    out.add(name(fi + df, rank + step))
            return out
        if kind == "N":
            for df, dr in KNIGHT:
                if on_board(fi + df, rank + dr):
                    out.add(name(fi + df, rank + dr))
            return out
        if kind == "K":
            for df, dr in ORTHO + DIAG:
                if on_board(fi + df, rank + dr):
                    out.add(name(fi + df, rank + dr))
            return out
        for r in self.rays(square):
            out.update(r.reach)
        return out

    def destinations(self, square) -> set[str]:
        """Squares this man can legally move to.

        Pseudo-legal unless both kings are recorded, in which case moves that
        leave the mover's own king attacked are dropped.
        """
        moves = self._pseudo_destinations(square)
        if not self.has_both_kings():
            return moves
        mover = self.men[square][0]
        legal = set()
        for target in moves:
            after = self.copy()
            after.men.pop(square)
            after.men[target] = self.men[square]
            king = after.squares_of(mover, "K")
            if not king or not after.attacked_by(king[0], opponent(mover)):
                legal.add(target)
        return legal

    def _pseudo_destinations(self, square) -> set[str]:
        man = self.men.get(square)
        if not man:
            return set()
        color, kind = man[0], man[1]
        if kind == "P":
            fi, rank = coords(square)
            step = 1 if color == "w" else -1
            start = 2 if color == "w" else 7
            out = set()
            ahead = name(fi, rank + step) if on_board(fi, rank + step) else None
            if ahead and ahead not in self.men:
                out.add(ahead)
                two = name(fi, rank + 2 * step) if on_board(fi, rank + 2 * step) else None
                if rank == start and two and two not in self.men:
                    out.add(two)
            for target in self.attacks(square):
                other = self.men.get(target)
                if other and other[0] != color:
                    out.add(target)
            return out
        return {t for t in self.attacks(square)
                if self.color_at(t) != color}

    def attacked_by(self, square, color) -> bool:
        return any(square in self.attacks(sq) for sq in self.squares_of(color))

    def controllers(self, square, color=None) -> list[str]:
        """Which men bear on a square. The basis of every control question."""
        return sorted(sq for sq in self.squares_of(color)
                      if square in self.attacks(sq))

    def mobility(self, color) -> dict[str, int]:
        return {sq: len(self.destinations(sq))
                for sq in sorted(self.squares_of(color))}

    def total_mobility(self, color) -> int:
        return sum(self.mobility(color).values())

    # -- applying moves --------------------------------------------------
    def apply(self, move: str) -> "Board":
        """Apply a move written as ``"b4b5"`` or ``"b4-b5"``. Deliberately
        coordinate notation: fragments have no reliable disambiguation context
        for SAN, and silent misreads are worse than verbose input."""
        origin, target = parse_move(move)
        if origin not in self.men:
            raise ValueError(f"no man on {origin}")
        after = self.copy()
        man = after.men.pop(origin)
        after.men[target] = man
        after.side_to_move = opponent(man[0])
        after.label = None
        return after

    def __repr__(self):
        return f"<Board {self.label or ''} {len(self.men)} men, {self.side_to_move} to move>"

    def ascii(self) -> str:
        rows = []
        for rank in range(8, 0, -1):
            cells = []
            for fi in range(8):
                man = self.men.get(name(fi, rank))
                cells.append(man[1] if man and man[0] == "w"
                             else man[1].lower() if man else ".")
            rows.append(f"{rank} " + " ".join(cells))
        return "\n".join(rows) + "\n  " + " ".join(FILES)

    # -- serialisation ---------------------------------------------------
    def to_dict(self):
        out = {"men": dict(sorted(self.men.items())), "side_to_move": self.side_to_move}
        if self.unrecorded:
            out["unrecorded"] = list(self.unrecorded)
        if self.label:
            out["label"] = self.label
        return out

    @classmethod
    def from_dict(cls, data):
        return cls(data.get("men"), data.get("side_to_move", "w"),
                   data.get("unrecorded"), data.get("label"))


def opponent(color: str) -> str:
    return "b" if color == "w" else "w"


def parse_move(move: str) -> tuple[str, str]:
    text = move.replace("-", "").replace("x", "").strip().lower()
    if len(text) != 4:
        raise ValueError(f"expected coordinate notation like 'b4b5', got {move!r}")
    return text[:2], text[2:]
