"""Holding games: observations, links, and bounded reachability.

A conventional chess app holds a game as a start position plus a move list --
a *path*. These notes are not paths. They are fragments recorded out of order,
some of them missing men that were certainly on the board. What is held is a
partially observed DAG: nodes are observations, edges are move sequences, and
most of the tree was never written down.

Three consequences shape this module.

1. An observation denotes a *set* of full positions, not one position. Its
   size is computable (``placement_space``), and every extra man recorded
   shrinks it. That shrinkage, in bits, is what a note is worth.

2. Edges are usually unknown. Attaching a later observation therefore means
   *searching* for move sequences that connect it to an earlier one, and
   reporting how many exist -- not asserting one.

3. Records are open by default: a man absent from a note was probably not
   written down, not captured. Checks must be one-sided. A note marked
   ``closed=True`` asserts completeness and unlocks the stricter rules.
"""

from __future__ import annotations

import json
import math
from itertools import permutations

from .board import Board, FILES, coords, opponent
from .metrics import evaluate_move


# ------------------------------------------------------- observations
class Observation:
    """One recorded position, plus what the recorder knew about it."""

    def __init__(self, key, board, note="", closed=False, source="", order=None):
        self.key = key
        self.board = board
        self.note = note
        self.closed = closed          # True: every man on the board is written down
        self.source = source
        self.order = order            # recorder's sequence, if known

    def __repr__(self):
        return f"<Observation {self.key} ({len(self.board.men)} men)>"

    def to_dict(self):
        out = {"key": self.key, "board": self.board.to_dict()}
        for field in ("note", "source"):
            if getattr(self, field):
                out[field] = getattr(self, field)
        if self.closed:
            out["closed"] = True
        if self.order is not None:
            out["order"] = self.order
        return out

    @classmethod
    def from_dict(cls, data):
        return cls(data["key"], Board.from_dict(data["board"]),
                   data.get("note", ""), data.get("closed", False),
                   data.get("source", ""), data.get("order"))


def placement_space(board: Board, roster=None) -> dict:
    """How many full positions this observation is consistent with.

    Counts the ways the unrecorded men could sit on the empty squares. Kings
    are handled exactly (adjacent kings are illegal) when the roster is small
    enough to enumerate; otherwise the falling factorial is used and the result
    is marked approximate.

    This is the information content of a note, made concrete: the log2 of this
    count is how many bits are still missing.
    """
    roster = list(roster if roster is not None else board.unrecorded)
    empty = board.empty_squares
    k, n = len(roster), len(empty)
    if k == 0:
        return {"roster": [], "placements": 1, "bits": 0.0, "exact": True}
    if k > n:
        raise ValueError("more unrecorded men than empty squares")

    ordered = 1
    for i in range(k):
        ordered *= (n - i)

    if ordered <= 2_000_000:
        kings = [i for i, man in enumerate(roster) if man[1] == "K"]
        count = 0
        for placement in permutations(empty, k):
            if len(kings) == 2:
                a, b = coords(placement[kings[0]]), coords(placement[kings[1]])
                if abs(a[0] - b[0]) <= 1 and abs(a[1] - b[1]) <= 1:
                    continue
            count += 1
        # identical men are interchangeable
        for man in set(roster):
            count //= math.factorial(roster.count(man))
        return {"roster": roster, "placements": count,
                "bits": math.log2(count) if count else 0.0, "exact": True}

    for man in set(roster):
        ordered //= math.factorial(roster.count(man))
    return {"roster": roster, "placements": ordered,
            "bits": math.log2(ordered), "exact": False}


# --------------------------------------------------------------- links
class LinkCheck:
    def __init__(self, violations, warnings):
        self.violations = violations
        self.warnings = warnings

    @property
    def possible(self):
        return not self.violations

    def __repr__(self):
        return f"<LinkCheck possible={self.possible} {len(self.warnings)} warnings>"


def link_check(parent: Observation, child: Observation) -> LinkCheck:
    """Cheap necessary conditions for ``child`` descending from ``parent``.

    Open records permit almost anything, so most findings here are warnings.
    A closed parent makes pawn monotonicity and material monotonicity binding,
    and those become violations.
    """
    violations, warnings = [], []
    pb, cb = parent.board, child.board
    strict = parent.closed and child.closed

    for color, direction in (("w", 1), ("b", -1)):
        for f in FILES:
            p_ranks = sorted(int(sq[1:]) for sq, man in pb.men.items()
                             if sq[0] == f and man == color + "P")
            c_ranks = sorted(int(sq[1:]) for sq, man in cb.men.items()
                             if sq[0] == f and man == color + "P")
            if not p_ranks or not c_ranks:
                continue
            p_edge = max(p_ranks) if direction == 1 else min(p_ranks)
            c_edge = max(c_ranks) if direction == 1 else min(c_ranks)
            retreated = (c_edge < p_edge) if direction == 1 else (c_edge > p_edge)
            if retreated:
                msg = (f"{color} pawn on file {f}: most advanced goes "
                       f"{f}{p_edge} -> {f}{c_edge}; pawns do not retreat")
                (violations if strict else warnings).append(msg)

    if strict:
        for color in ("w", "b"):
            if len(cb.squares_of(color)) > len(pb.squares_of(color)):
                violations.append(
                    f"{color} gains men ({len(pb.squares_of(color))} -> "
                    f"{len(cb.squares_of(color))}) between closed records")
    else:
        for color in ("w", "b"):
            extra = len(cb.squares_of(color)) - len(pb.squares_of(color))
            if extra > 0:
                warnings.append(
                    f"{color} shows {extra} more men than the parent record; "
                    f"open record, so probably newly written down rather than new")
    return LinkCheck(violations, warnings)


def _requirements(child: Board, parent: Board) -> list[tuple[str, str]]:
    """(square, man) pairs the child asserts that the parent does not match."""
    return sorted((sq, man) for sq, man in child.men.items()
                  if parent.men.get(sq) != man)


def find_paths(parent: Observation, child: Observation, max_plies=4,
               limit=20, side_to_move=None) -> list[list[str]]:
    """Exact bounded search for move sequences from parent to child.

    A path succeeds when every man the child records is standing where the
    child records it. Men the child does not mention are unconstrained -- that
    is what makes the record open.

    Depth is capped because the search is exponential; the point is not to
    reconstruct the whole game but to tell you whether two notes can be four
    moves apart, and if so, how.
    """
    start = parent.board.copy()
    if side_to_move:
        start.side_to_move = side_to_move
    target = child.board
    found = []

    def satisfied(board):
        return all(board.men.get(sq) == man for sq, man in target.men.items())

    def walk(board, depth, path):
        if len(found) >= limit:
            return
        if satisfied(board):
            found.append(list(path))
            return
        if depth == 0:
            return
        if len(_requirements(target, board)) > depth:
            return                      # cannot fix that many squares in time
        for origin in sorted(board.squares_of(board.side_to_move)):
            for dest in sorted(board.destinations(origin)):
                move = f"{origin}{dest}"
                walk(board.apply(move), depth - 1, path + [move])
                if len(found) >= limit:
                    return

    walk(start, max_plies, [])
    return found


# -------------------------------------------------------------- growth
class Ply:
    __slots__ = ("index", "move", "scope", "opponent_scope", "optionality",
                 "unlocked", "imported", "commitment", "reversible")

    def __init__(self, **kw):
        for k in self.__slots__:
            setattr(self, k, kw.get(k))


class Branch:
    """A named line from a node, as a list of ``evaluate_move`` kwargs."""

    def __init__(self, name, moves, note=""):
        self.name = name
        self.moves = moves
        self.note = note

    def to_dict(self):
        out = {"name": self.name, "moves": self.moves}
        if self.note:
            out["note"] = self.note
        return out

    @classmethod
    def from_dict(cls, data):
        return cls(data["name"], data["moves"], data.get("note", ""))


def growth_profile(board: Board, branch: Branch, color="w") -> list[Ply]:
    """Walk a branch, recording scope, commitment and optionality per ply.

    Balanced growth is not maximal growth. The three columns move
    independently: scope can rise while optionality collapses, and commitment
    only ever rises. Reading them together is the comparison.
    """
    current = board.copy()
    plies = [Ply(index=0, move="(start)",
                 scope=current.total_mobility(color),
                 opponent_scope=current.total_mobility(opponent(color)),
                 optionality=current.total_mobility(color),
                 unlocked=0, imported=0, commitment=0, reversible=True)]
    unlocked = imported = commitment = 0
    for i, spec in enumerate(branch.moves, start=1):
        ev = evaluate_move(current, **spec)
        unlocked += ev.unlocked
        imported += ev.imported
        if not ev.reversible:
            commitment += 1
        current = ev.after
        plies.append(Ply(index=i, move=ev.move,
                         scope=current.total_mobility(color),
                         opponent_scope=current.total_mobility(opponent(color)),
                         optionality=current.total_mobility(color),
                         unlocked=unlocked, imported=imported,
                         commitment=commitment, reversible=ev.reversible))
    return plies


def branch_summary(board: Board, branch: Branch, color="w") -> dict:
    plies = growth_profile(board, branch, color)
    last, first = plies[-1], plies[0]
    gained = last.scope - first.scope
    return {
        "name": branch.name,
        "plies": len(branch.moves),
        "scope_start": first.scope,
        "scope_end": last.scope,
        "scope_gained": gained,
        "unlocked": last.unlocked,
        "imported": last.imported,
        "commitment": last.commitment,
        # scope bought per irreversible decision; None when nothing was spent
        "balance": None if last.commitment == 0 else gained / last.commitment,
        "profile": plies,
        "note": branch.note,
    }


# --------------------------------------------------------------- study
class Study:
    """A held game: observations, the links between them, and named branches."""

    def __init__(self, name="", observations=None, links=None, branches=None,
                 note=""):
        self.name = name
        self.note = note
        self.observations = dict(observations or {})
        self.links = list(links or [])          # {"from":k,"to":k,"moves":[...] }
        self.branches = dict(branches or {})    # root key -> [Branch]

    def add(self, obs: Observation):
        self.observations[obs.key] = obs
        return obs

    def parents_of(self, key):
        return [l["from"] for l in self.links if l["to"] == key]

    def children_of(self, key):
        return [l["to"] for l in self.links if l["from"] == key]

    def roots(self):
        return [k for k in self.observations if not self.parents_of(k)]

    def attach(self, obs: Observation, max_plies=4) -> list[dict]:
        """Where could a newly recorded position sit?

        Runs the cheap check against every held observation, then searches for
        concrete move sequences where the check allows it. Returns candidates
        ranked by shortest path found -- it proposes, it does not commit.
        """
        candidates = []
        for key, other in self.observations.items():
            if key == obs.key:
                continue
            check = link_check(other, obs)
            entry = {"parent": key, "possible": check.possible,
                     "violations": check.violations, "warnings": check.warnings,
                     "paths": []}
            if check.possible:
                entry["paths"] = find_paths(other, obs, max_plies=max_plies, limit=5)
            entry["shortest"] = min((len(p) for p in entry["paths"]), default=None)
            candidates.append(entry)
        candidates.sort(key=lambda c: (not c["possible"],
                                       c["shortest"] if c["shortest"] is not None else 99))
        return candidates

    # -- io --------------------------------------------------------------
    def to_dict(self):
        return {
            "name": self.name,
            "note": self.note,
            "observations": [o.to_dict() for o in self.observations.values()],
            "links": self.links,
            "branches": {k: [b.to_dict() for b in v] for k, v in self.branches.items()},
        }

    @classmethod
    def from_dict(cls, data):
        obs = {o["key"]: Observation.from_dict(o) for o in data.get("observations", [])}
        branches = {k: [Branch.from_dict(b) for b in v]
                    for k, v in data.get("branches", {}).items()}
        return cls(data.get("name", ""), obs, data.get("links"), branches,
                   data.get("note", ""))

    @classmethod
    def load(cls, path):
        with open(path) as fh:
            return cls.from_dict(json.load(fh))

    def save(self, path):
        with open(path, "w") as fh:
            json.dump(self.to_dict(), fh, indent=2)
            fh.write("\n")
