"""Text reports. The analysis a case study would otherwise be written by hand."""

from __future__ import annotations

from .board import Board, opponent
from .metrics import (beams, self_blocked_beams, compare, coupling_table)
from .tree import Observation, Study, branch_summary, placement_space

RULE = "-" * 74


def _head(text):
    return f"\n{text}\n{RULE}"


def position_report(obs: Observation, candidates=None, color="w") -> str:
    board = obs.board
    out = [_head(f"{obs.key}  {obs.note or ''}".rstrip()), board.ascii(), ""]

    out.append(f"{'':2}side to move: {board.side_to_move}   "
               f"record: {'closed' if obs.closed else 'open'}")
    if board.unrecorded:
        space = placement_space(board)
        qualifier = "" if space["exact"] else " (approx)"
        out.append(f"{'':2}unrecorded: {', '.join(board.unrecorded)}  ->  "
                   f"{space['placements']:,} positions consistent"
                   f"{qualifier}, {space['bits']:.1f} bits missing")

    out.append(_head("SCOPE"))
    for side in ("w", "b"):
        per = board.mobility(side)
        detail = "  ".join(f"{sq}{board.men[sq][1]}:{n}" for sq, n in per.items())
        out.append(f"  {side}  total {sum(per.values()):>3}   {detail}")

    out.append(_head("BEAMS  (long-range pieces and what stops them)"))
    any_beam = False
    for beam in beams(board):
        if beam.length == 0 and beam.blocker is None:
            continue
        any_beam = True
        flag = "  <-- self-blocked" if beam.self_blocked else ""
        out.append(f"  {beam.piece}  {beam.describe()}{flag}")
    if not any_beam:
        out.append("  (no sliding pieces with reach)")

    out.append(_head("COUPLING  (systems each square participates in)"))
    out.append(f"  {'sq':<4}{'man':<5}{'static':>7}{'latent':>8}   systems")
    for row in coupling_table(board):
        out.append(f"  {row['square']:<4}{row['man']:<5}{row['static']:>7}"
                   f"{row['latent']:>8}   {', '.join(row['systems']) or '-'}")
    out.append("")
    out.append("  static = live now.  latent = switches on within one move.")
    out.append("  A high-latent, low-static square is loaded but not yet firing.")

    if candidates:
        out.append(_head(f"CANDIDATES  (for {color}, ordered by the commitment rule)"))
        out.append(f"  {'move':<10} {'scope':>10}          unlocked  imported  unlock  cost")
        for ev in compare(board, candidates):
            out.append("  " + ev.summary())
        out.append("")
        out.append("  Ordering is reversible-first, then by scope unlocked --")
        out.append("  not by raw gain. Free scope outranks bought scope.")
    return "\n".join(out)


def branch_report(board: Board, branches, color="w") -> str:
    out = [_head("BRANCHES")]
    summaries = [branch_summary(board, b, color) for b in branches]
    out.append(f"  {'branch':<24}{'plies':>6}{'scope':>10}{'unlock':>8}"
               f"{'import':>8}{'commit':>8}{'balance':>9}")
    for s in summaries:
        balance = "free" if s["balance"] is None else f"{s['balance']:.1f}"
        out.append(f"  {s['name']:<24}{s['plies']:>6}"
                   f"{str(s['scope_start']) + '->' + str(s['scope_end']):>10}"
                   f"{s['unlocked']:>8}{s['imported']:>8}"
                   f"{s['commitment']:>8}{balance:>9}")
    out.append("")
    out.append("  balance = scope gained per irreversible decision.")
    out.append("  'free' means no commitment was spent at all -- not that it is best,")
    out.append("  only that nothing was permanently given up to get there.")

    for s in summaries:
        out.append(_head(f"PROFILE  {s['name']}"))
        if s["note"]:
            out.append(f"  {s['note']}\n")
        out.append(f"  {'ply':<4}{'move':<12}{'scope':>7}{'opp':>6}"
                   f"{'unlock':>8}{'import':>8}{'commit':>8}")
        for ply in s["profile"]:
            out.append(f"  {ply.index:<4}{ply.move:<12}{ply.scope:>7}"
                       f"{ply.opponent_scope:>6}{ply.unlocked:>8}"
                       f"{ply.imported:>8}{ply.commitment:>8}")
    return "\n".join(out)


def study_report(study: Study, color="w") -> str:
    out = [RULE, f"STUDY: {study.name}", RULE]
    if study.note:
        out.append(study.note)
    for key, obs in study.observations.items():
        candidates = getattr(obs, "candidates", None)
        out.append(position_report(obs, candidates, color))
        for branch_list in [study.branches.get(key, [])]:
            if branch_list:
                out.append(branch_report(obs.board, branch_list, color))
    if study.links:
        out.append(_head("LINKS"))
        for link in study.links:
            moves = link.get("moves")
            label = " ".join(moves) if moves else f"unknown, >= {link.get('min_plies','?')} plies"
            out.append(f"  {link['from']} -> {link['to']}   {label}")
    return "\n".join(out) + "\n"
