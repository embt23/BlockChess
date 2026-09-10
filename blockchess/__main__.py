"""CLI:  python -m blockchess report games/01-self-play/study.json
         python -m blockchess note "white [pawn: A5, B4/bishop: A3]"
         python -m blockchess attach games/01-self-play/study.json new.json
"""

from __future__ import annotations

import json
import sys

from .notation import parse_note
from .report import position_report, study_report
from .tree import Observation, Study


def main(argv=None):
    argv = list(argv if argv is not None else sys.argv[1:])
    if not argv:
        print(__doc__)
        return 1
    command, rest = argv[0], argv[1:]

    if command == "report":
        if not rest:
            print("usage: report <study.json>")
            return 1
        print(study_report(Study.load(rest[0])))
        return 0

    if command == "note":
        if not rest:
            print('usage: note "white [pawn: A5, B4/bishop: A3]"')
            return 1
        board = parse_note(rest[0], unrecorded=["wK", "bK"])
        print(position_report(Observation("note", board)))
        return 0

    if command == "attach":
        if len(rest) < 2:
            print("usage: attach <study.json> <observation.json>")
            return 1
        study = Study.load(rest[0])
        with open(rest[1]) as fh:
            obs = Observation.from_dict(json.load(fh))
        print(f"Where can {obs.key} attach?\n")
        for cand in study.attach(obs):
            verdict = "possible" if cand["possible"] else "RULED OUT"
            print(f"  from {cand['parent']}: {verdict}")
            for v in cand["violations"]:
                print(f"      violation: {v}")
            for w in cand["warnings"]:
                print(f"      warning:   {w}")
            if cand["paths"]:
                print(f"      {len(cand['paths'])} path(s), shortest {cand['shortest']} plies:")
                for path in cand["paths"][:3]:
                    print(f"        {' '.join(path)}")
            elif cand["possible"]:
                print("      no path found within the ply budget")
        return 0

    print(f"unknown command {command!r}")
    print(__doc__)
    return 1


if __name__ == "__main__":
    raise SystemExit(main())
