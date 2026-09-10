"""Position 1 is the reference fixture.

Case study 01 states numbers in prose. These tests pin them to the engine, so
that if the measures change, the prose is known to be stale rather than
quietly wrong.
"""

import unittest

from blockchess import (Board, Observation, Study, Branch, parse_note, to_note,
                        compare, coupling_table, evaluate_move, find_paths,
                        link_check, placement_space, self_blocked_beams,
                        square_color, systems_at, branch_summary)

NOTE = "white [pawn: A5, B4/bishop: A3] black [pawns D5, E5/bishop D7/knight C6]"


def position_one():
    return parse_note(NOTE, side_to_move="w", unrecorded=["wK", "bK"],
                      label="Position 1")


class Geometry(unittest.TestCase):
    def test_a3_f8_diagonal_is_all_dark(self):
        for sq in ("a3", "b4", "c5", "d6", "e7", "f8"):
            self.assertEqual(square_color(sq), "dark", sq)

    def test_the_two_bishops_never_meet(self):
        board = position_one()
        self.assertEqual(square_color("a3"), "dark")
        self.assertEqual(square_color("d7"), "light")
        self.assertNotEqual(square_color("a3"), square_color("d7"))

    def test_e5_is_dark_and_d7_bishop_can_never_defend_it(self):
        self.assertEqual(square_color("e5"), "dark")
        board = position_one()
        self.assertNotIn("e5", board.attacks("d7"))


class Notation(unittest.TestCase):
    def test_the_notebook_line_parses(self):
        board = position_one()
        self.assertEqual(board.men, {
            "a5": "wP", "b4": "wP", "a3": "wB",
            "d5": "bP", "e5": "bP", "d7": "bB", "c6": "bN"})

    def test_round_trip(self):
        board = position_one()
        self.assertEqual(parse_note(to_note(board)).men, board.men)


class Scope(unittest.TestCase):
    def test_mobility_totals(self):
        board = position_one()
        self.assertEqual(board.total_mobility("w"), 4)
        self.assertEqual(board.total_mobility("b"), 15)

    def test_per_man_mobility(self):
        board = position_one()
        self.assertEqual(board.mobility("w"), {"a3": 2, "a5": 1, "b4": 1})
        self.assertEqual(board.mobility("b"),
                         {"c6": 7, "d5": 1, "d7": 6, "e5": 1})

    def test_bishop_is_blocked_by_its_own_pawn(self):
        blocked = self_blocked_beams(position_one(), "w")
        self.assertEqual(len(blocked), 1)
        self.assertEqual(blocked[0].origin, "a3")
        self.assertEqual(blocked[0].blocker, "b4")

    def test_the_only_defender_of_e5_is_the_knight(self):
        board = position_one()
        self.assertEqual(board.controllers("e5", "b"), ["c6"])


class Coupling(unittest.TestCase):
    def test_b4_carries_the_most_load(self):
        rows = coupling_table(position_one())
        self.assertEqual(rows[0]["square"], "b4")
        self.assertEqual(rows[0]["static"], 4)

    def test_b4_participates_in_these_systems(self):
        self.assertEqual(sorted(systems_at(position_one(), "b4")),
                         ["blocks_friendly_line", "defends", "lever",
                          "sole_control"])

    def test_the_central_pawns_are_latent_not_live(self):
        """d5 and e5 do almost nothing now and light up within one move.
        This is the measure independently finding the Bb2 idea."""
        rows = {r["square"]: r for r in coupling_table(position_one())}
        for square in ("d5", "e5"):
            self.assertEqual(rows[square]["static"], 1)
            self.assertGreater(rows[square]["latent"], rows[square]["static"])


class Candidates(unittest.TestCase):
    def test_bb2_attacks_e5_without_moving_the_pawn(self):
        after = position_one().apply("a3b2")
        self.assertIn("e5", after.attacks("b2"))
        self.assertEqual(after.men["b4"], "wP")

    def test_bb2_is_free_scope(self):
        ev = evaluate_move(position_one(), move="a3b2")
        self.assertEqual((ev.before_total, ev.after_total), (4, 8))
        self.assertEqual((ev.unlocked, ev.imported), (4, 0))
        self.assertTrue(ev.reversible)

    def test_b5_buys_more_but_is_irreversible(self):
        ev = evaluate_move(position_one(), move="b4b5")
        self.assertEqual((ev.before_total, ev.after_total), (4, 10))
        self.assertFalse(ev.reversible)
        self.assertEqual(ev.commitment_reason, "pawn move")

    def test_nc3_imports_everything_and_unlocks_nothing(self):
        ev = evaluate_move(position_one(), introduce="wN", at="c3")
        self.assertEqual(ev.unlocked, 0)
        self.assertEqual(ev.imported, 8)
        self.assertEqual(ev.unlock_fraction, 0.0)

    def test_ra3_is_illegal_while_the_bishop_stands_there(self):
        with self.assertRaises(ValueError):
            evaluate_move(position_one(), introduce="wR", at="a3")

    def test_ordering_puts_free_scope_first_and_commitment_last(self):
        ranked = compare(position_one(), [
            {"move": "b4b5"}, {"move": "a3b2"}, {"introduce": "wN", "at": "c3"}])
        self.assertEqual(ranked[0].move, "a3b2")
        self.assertFalse(ranked[-1].reversible)

    def test_b5_evicts_the_only_defender_of_e5(self):
        """After Bb2 and b5 the knight is attacked, and no square it can reach
        still defends e5."""
        board = position_one().apply("a3b2").apply("b4b5")
        self.assertIn("c6", board.attacks("b5"))
        for dest in board.destinations("c6"):
            after = board.apply(f"c6{dest}")
            self.assertNotIn("e5", after.attacks(dest),
                             f"...N{dest} unexpectedly still holds e5")

    def test_d4_is_the_defensive_resource(self):
        """...Nd4 does not defend e5 -- it blocks the diagonal instead, which
        is why denying d4 is the point of e3."""
        board = position_one().apply("a3b2")
        after = board.apply("c6d4")
        self.assertNotIn("e5", after.attacks("d4"))
        self.assertNotIn("e5", after.attacks("b2"))       # diagonal now blocked
        self.assertIn("d4", board.attacks("e5"))          # and it is supported


class Holding(unittest.TestCase):
    def test_placement_space_is_exact_for_two_kings(self):
        space = placement_space(position_one())
        self.assertTrue(space["exact"])
        self.assertEqual(space["placements"], 2862)
        self.assertAlmostEqual(space["bits"], 11.48, places=1)

    def test_recording_a_king_shrinks_the_space(self):
        board = position_one()
        board.men["g1"] = "wK"
        board.unrecorded = ["bK"]
        self.assertLess(placement_space(board)["bits"],
                        placement_space(position_one())["bits"])

    def test_open_records_do_not_rule_out_extra_men(self):
        parent = Observation("p1", position_one())
        richer = position_one()
        richer.men["g8"] = "bR"
        check = link_check(parent, Observation("p2", richer))
        self.assertTrue(check.possible)
        self.assertTrue(check.warnings)

    def test_closed_records_forbid_retreating_pawns(self):
        parent = Observation("p1", position_one(), closed=True)
        back = position_one()
        back.men.pop("b4")
        back.men["b3"] = "wP"
        check = link_check(parent, Observation("p2", back, closed=True))
        self.assertFalse(check.possible)
        self.assertTrue(any("do not retreat" in v for v in check.violations))

    def test_find_paths_recovers_a_known_sequence(self):
        parent = Observation("p1", position_one())
        child = Observation("p2", position_one().apply("b4b5").apply("c6d4"))
        paths = find_paths(parent, child, max_plies=2)
        self.assertIn(["b4b5", "c6d4"], paths)

    def test_find_paths_returns_nothing_for_an_unreachable_note(self):
        parent = Observation("p1", position_one())
        impossible = position_one()
        impossible.men["h8"] = "wQ"
        paths = find_paths(parent, Observation("p2", impossible), max_plies=3)
        self.assertEqual(paths, [])


class Growth(unittest.TestCase):
    def test_keeping_the_gate_spends_less_than_pushing_it(self):
        board = position_one()
        push = branch_summary(board, Branch("A", [{"move": "b4b5"}]))
        keep = branch_summary(board, Branch("B", [{"move": "a3b2"}]))
        self.assertEqual(push["commitment"], 1)
        self.assertEqual(keep["commitment"], 0)
        self.assertGreater(keep["unlocked"], 0)

    def test_the_rook_on_a3_costs_the_b2_bishop_a_square(self):
        """Law 02 again: a3 is contested even after the bishop leaves it."""
        board = position_one().apply("a3b2")
        before = board.total_mobility("w")
        ev = evaluate_move(board, introduce="wR", at="a3")
        self.assertLess(ev.per_man["b2"][1], 0)

    def test_study_round_trips_through_json(self):
        study = Study(name="t", observations={"p1": Observation("p1", position_one())},
                      branches={"p1": [Branch("A", [{"move": "a3b2"}])]})
        again = Study.from_dict(study.to_dict())
        self.assertEqual(again.observations["p1"].board.men,
                         study.observations["p1"].board.men)
        self.assertEqual(again.branches["p1"][0].moves, [{"move": "a3b2"}])


if __name__ == "__main__":
    unittest.main(verbosity=2)
