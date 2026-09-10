"""BlockChess -- structural analysis of partially-recorded chess positions.

    from blockchess import parse_note, Board, Study

    board = parse_note("white [pawn: A5, B4/bishop: A3] black [knight: C6]")
    board.total_mobility("w")
"""

from .board import Board, square_color, opponent
from .notation import parse_note, to_note, NoteError
from .metrics import (beams, self_blocked_beams, coupling, coupling_table,
                      systems_at, latent_systems, evaluate_move, compare)
from .tree import (Observation, Study, Branch, placement_space, link_check,
                   find_paths, growth_profile, branch_summary)

__all__ = [
    "Board", "square_color", "opponent",
    "parse_note", "to_note", "NoteError",
    "beams", "self_blocked_beams", "coupling", "coupling_table",
    "systems_at", "latent_systems", "evaluate_move", "compare",
    "Observation", "Study", "Branch", "placement_space", "link_check",
    "find_paths", "growth_profile", "branch_summary",
]
__version__ = "0.1.0"
