"""Read the notes as they were actually written.

The recorded format is the one used in the notebook:

    white [pawn: A5, B4/bishop: A3] black [pawns D5, E5/bishop D7/knight C6]

Groups are separated by ``/``, the piece word may be singular or plural, the
colon is optional, and squares are case-insensitive. Keeping the parser
tolerant matters more than keeping it strict: the notes exist already, and a
format that requires rewriting them will not get used.
"""

from __future__ import annotations

import re

from .board import ALL_SQUARES, Board

KIND_WORDS = {
    "pawn": "P", "pawns": "P",
    "knight": "N", "knights": "N",
    "bishop": "B", "bishops": "B",
    "rook": "R", "rooks": "R",
    "queen": "Q", "queens": "Q",
    "king": "K", "kings": "K",
}
SIDE_WORDS = {"white": "w", "w": "w", "black": "b", "b": "b"}
SQUARE_RE = re.compile(r"^[a-h][1-8]$")


class NoteError(ValueError):
    pass


def parse_note(text: str, side_to_move="w", unrecorded=None, label=None) -> Board:
    """Parse one recorded position. Raises NoteError with the offending token."""
    men = {}
    for side_word, body in _side_blocks(text):
        color = SIDE_WORDS[side_word]
        for group in body.split("/"):
            group = group.strip()
            if not group:
                continue
            kind, squares = _parse_group(group)
            for square in squares:
                if square in men:
                    raise NoteError(f"{square} is claimed twice")
                men[square] = color + kind
    if not men:
        raise NoteError("no men found; expected e.g. 'white [pawn: A5]'")
    return Board(men, side_to_move, unrecorded, label)


def _side_blocks(text):
    blocks = re.findall(r"(white|black|\bw\b|\bb\b)\s*\[([^\]]*)\]",
                        text, flags=re.IGNORECASE)
    if not blocks:
        raise NoteError("expected 'white [...]' and/or 'black [...]'")
    return [(side.lower(), body) for side, body in blocks]


def _parse_group(group):
    head, _, tail = group.partition(":")
    if tail:
        word, rest = head.strip(), tail
    else:
        parts = group.split(None, 1)
        if len(parts) != 2:
            raise NoteError(f"cannot read group {group!r}")
        word, rest = parts

    kind = KIND_WORDS.get(word.strip().lower())
    if kind is None:
        raise NoteError(f"unknown piece word {word.strip()!r}")

    squares = []
    for token in re.split(r"[,\s]+", rest.strip()):
        if not token:
            continue
        square = token.strip().lower()
        if not SQUARE_RE.match(square):
            raise NoteError(f"{token!r} is not a square")
        squares.append(square)
    if not squares:
        raise NoteError(f"no squares given for {word.strip()!r}")
    return kind, squares


def to_note(board: Board) -> str:
    """Write a board back out in the notebook format, so the round trip holds."""
    order = ["K", "Q", "R", "B", "N", "P"]
    chunks = []
    for color, word in (("w", "white"), ("b", "black")):
        groups = []
        for kind in order:
            squares = sorted(board.squares_of(color, kind),
                             key=lambda s: (s[0], s[1]))
            if not squares:
                continue
            label = [w for w, k in KIND_WORDS.items()
                     if k == kind and w.endswith("s") == (len(squares) > 1)][0]
            groups.append(f"{label}: {', '.join(s.upper() for s in squares)}")
        if groups:
            chunks.append(f"{word} [{'/'.join(groups)}]")
    return " ".join(chunks)
