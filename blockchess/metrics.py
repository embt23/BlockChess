"""Measures over a fragment.

Four measures, each answering a question the notes kept asking in prose:

  beams      what stops each long-range piece, and whose fault it is
  coupling   how much structural load a square carries (Law 03)
  scope      how much mobility a move unlocks vs. merely imports
  commitment whether a move can be taken back

Coupling is reported in two forms. *Static* coupling counts systems live in the
position as recorded. *Latent* coupling counts systems that switch on somewhere
in the one-move neighbourhood -- the difference between a square that is doing
work now and one that is waiting to.
"""

from __future__ import annotations

from .board import Board, SLIDE, coords, name, on_board, opponent

SYSTEMS = ("blocks_friendly_line", "blocks_enemy_line",
           "defends", "sole_control", "lever")


# ---------------------------------------------------------------- beams
class Beam:
    __slots__ = ("piece", "origin", "direction", "open_squares",
                 "blocker", "blocker_man", "self_blocked")

    def __init__(self, piece, ray):
        self.piece = piece
        self.origin = ray.origin
        self.direction = ray.direction
        self.open_squares = ray.open_squares
        self.blocker = ray.blocker
        self.blocker_man = ray.blocker_man
        self.self_blocked = ray.blocked_by_own

    @property
    def length(self):
        return len(self.open_squares)

    def describe(self):
        if self.blocker is None:
            tail = "runs to the edge"
        elif self.self_blocked:
            tail = f"stopped by own {self.blocker_man} on {self.blocker}"
        else:
            tail = f"bears on {self.blocker_man} on {self.blocker}"
        squares = " ".join(self.open_squares) or "-"
        return f"{self.origin}: {squares} ({tail})"


def beams(board: Board, color=None) -> list[Beam]:
    """Every ray of every sliding piece, with what stops it.

    Law 01: a long-range piece's value is a property of its line, not its
    square. This is the function that makes that checkable.
    """
    out = []
    for square in sorted(board.squares_of(color)):
        man = board.men[square]
        if man[1] not in SLIDE:
            continue
        for ray in board.rays(square):
            out.append(Beam(man, ray))
    return out


def self_blocked_beams(board: Board, color) -> list[Beam]:
    """The lines you are standing on yourself -- Law 02's evidence."""
    return [b for b in beams(board, color)
            if b.self_blocked and _extends_past(board, b)]


def _extends_past(board, beam) -> bool:
    """Would clearing the blocker actually buy reach, or is it at the edge?"""
    if beam.blocker is None:
        return False
    fi, rank = coords(beam.blocker)
    df, dr = beam.direction
    return on_board(fi + df, rank + dr)


# ------------------------------------------------------------- coupling
def systems_at(board: Board, square: str) -> list[str]:
    """Which of the five systems the man on ``square`` currently participates in."""
    man = board.men.get(square)
    if not man:
        return []
    color = man[0]
    active = []

    friendly_block = enemy_block = False
    for other in board.squares_of():
        if other == square or board.men[other][1] not in SLIDE:
            continue
        for ray in board.rays(other):
            if ray.blocker == square and _extends_past(board, Beam(board.men[other], ray)):
                if board.men[other][0] == color:
                    friendly_block = True
                else:
                    enemy_block = True
    if friendly_block:
        active.append("blocks_friendly_line")
    if enemy_block:
        active.append("blocks_enemy_line")

    if any(board.color_at(t) == color for t in board.attacks(square)):
        active.append("defends")

    for target in board.attacks(square):
        if board.controllers(target) == [square]:
            active.append("sole_control")
            break

    if man[1] == "P" and _is_lever(board, square):
        active.append("lever")

    return active


def _is_lever(board: Board, square: str) -> bool:
    """A pawn whose next step would bear on an enemy man."""
    man = board.men[square]
    for target in board.destinations(square):
        after = board.apply(f"{square}{target}")
        if any(after.color_at(t) == opponent(man[0]) for t in after.attacks(target)):
            return True
    return False


def coupling(board: Board, square: str) -> int:
    return len(systems_at(board, square))


def latent_systems(board: Board, square: str, color=None) -> list[str]:
    """Systems that switch on anywhere in the one-move neighbourhood.

    Moves by the man on ``square`` are excluded -- if it leaves, the question
    is moot. Everything else the given side could play is tried.
    """
    seen = set(systems_at(board, square))
    for origin in board.squares_of(color if color is not None else board.side_to_move):
        if origin == square:
            continue
        for target in board.destinations(origin):
            if target == square:
                continue
            seen.update(systems_at(board.apply(f"{origin}{target}"), square))
    return sorted(seen)


def coupling_table(board: Board, latent=True) -> list[dict]:
    """Every occupied square, ranked by static coupling. Ties break on latent."""
    rows = []
    for square in board.men:
        static = systems_at(board, square)
        row = {
            "square": square,
            "man": board.men[square],
            "static": len(static),
            "systems": static,
        }
        if latent:
            lat = set(latent_systems(board, square, "w"))
            lat |= set(latent_systems(board, square, "b"))
            row["latent"] = len(lat)
            row["latent_systems"] = sorted(lat)
        rows.append(row)
    rows.sort(key=lambda r: (-r["static"], -r.get("latent", 0), r["square"]))
    return rows


# ---------------------------------------------------------------- scope
class MoveEval:
    """What a candidate move does to your own scope, and what it costs.

    ``unlocked`` is mobility gained by men already recorded in the fragment.
    ``imported`` is mobility carried in by a man arriving from off-record.
    They are reported separately because they are not the same currency:
    importing is linear in pieces developed, unlocking is structural.
    """

    __slots__ = ("move", "color", "before_total", "after_total", "unlocked",
                 "imported", "reversible", "commitment_reason", "per_man", "after")

    def __init__(self, **kw):
        for k in self.__slots__:
            setattr(self, k, kw.get(k))

    @property
    def delta(self):
        return self.after_total - self.before_total

    @property
    def unlock_fraction(self):
        gained = self.unlocked + self.imported
        return None if gained <= 0 else self.unlocked / gained

    def summary(self):
        frac = self.unlock_fraction
        frac_text = "n/a" if frac is None else f"{frac:.0%}"
        cost = "reversible" if self.reversible else f"IRREVERSIBLE ({self.commitment_reason})"
        return (f"{self.move:<10} {self.before_total:>3} -> {self.after_total:<3} "
                f"(D{self.delta:+d})  unlocked {self.unlocked:>2}  "
                f"imported {self.imported:>2}  unlock {frac_text:>4}  {cost}")


def evaluate_move(board: Board, move: str = None, introduce: str = None,
                  at: str = None) -> MoveEval:
    """Score a candidate.

    Either relocate a recorded man (``move="a3b2"``) or bring one in from
    off-record (``introduce="wN", at="c3"``). The second form is how a fragment
    represents development: the knight was never written down, so its scope is
    imported rather than unlocked.
    """
    if move:
        origin, target = move[:2], move[2:4]
        man = board.men.get(origin)
        if not man:
            raise ValueError(f"no man on {origin}")
        color = man[0]
        after = board.apply(move)
        predecessor = {target: origin}
        label = move
        captured = board.at(target)
        reversible = man[1] != "P" and captured is None
        reason = "pawn move" if man[1] == "P" else ("capture" if captured else "")
    elif introduce and at:
        color = introduce[0]
        if at in board.men:
            raise ValueError(f"{at} is occupied")
        after = board.copy()
        after.men[at] = introduce
        after.side_to_move = opponent(color)
        predecessor = {}
        label = f"{introduce}@{at}"
        # a pawn arriving on a square got there by a pawn move, which no
        # amount of later play can take back
        is_pawn = introduce[1] == "P"
        reversible = not is_pawn
        reason = "pawn move" if is_pawn else ""
    else:
        raise ValueError("pass either move= or introduce=/at=")

    before_mob = board.mobility(color)
    after_mob = after.mobility(color)
    unlocked = imported = 0
    per_man = {}
    for square, count in after_mob.items():
        origin_sq = predecessor.get(square, square)
        was = before_mob.get(origin_sq)
        if was is None:
            imported += count
            per_man[square] = ("imported", count)
        else:
            unlocked += count - was
            per_man[square] = ("unlocked", count - was)
    for square, was in before_mob.items():
        if square not in after_mob and square not in predecessor.values():
            unlocked -= was
            per_man[square] = ("lost", -was)

    return MoveEval(move=label, color=color,
                    before_total=sum(before_mob.values()),
                    after_total=sum(after_mob.values()),
                    unlocked=unlocked, imported=imported,
                    reversible=reversible, commitment_reason=reason,
                    per_man=per_man, after=after)


def compare(board: Board, candidates: list[dict]) -> list[MoveEval]:
    """Score a slate of candidates.

    Ordering is the rule from case study 01, not raw gain: reversible moves
    first, then by scope unlocked per unit of commitment. A move that buys
    scope for free outranks one that buys more for a pawn move.
    """
    evals = [evaluate_move(board, **c) for c in candidates]
    evals.sort(key=lambda e: (e.reversible is False, -e.unlocked, -e.delta))
    return evals
