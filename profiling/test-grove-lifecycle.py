#!/usr/bin/env python3
"""Grove lifecycle route uses the published Area 1746 ridge, not +Z."""
import importlib.util
import json
import tempfile
import unittest
from pathlib import Path

spec = importlib.util.spec_from_file_location(
    "grove_lifecycle", Path(__file__).with_name("grove-lifecycle.py")
)
lifecycle = importlib.util.module_from_spec(spec)
spec.loader.exec_module(lifecycle)


class GroveLifecycleRouteTests(unittest.TestCase):
    def gotos(self, census=False):
        return [row for row in lifecycle.route(census) if row["cmd"] == "goto"]

    def test_starts_at_area_1746(self):
        start = lifecycle.route()[0]
        self.assertEqual(start["cmd"], "start")
        self.assertEqual(start["xz"], list(lifecycle.AREA_1746))

    def test_rejects_the_failed_plus_z_first_hop(self):
        first = self.gotos()[0]["xz"]
        self.assertNotEqual(tuple(first), lifecycle.FAILED_PLUS_Z_FIRST)
        self.assertEqual(first, [1760.535, 1060.718])

    def test_stays_on_the_published_ridge(self):
        gotos = self.gotos()
        self.assertEqual(gotos[0]["xz"], [1760.535, 1060.718])
        self.assertEqual(gotos[len(lifecycle.APPROACH) - 1]["xz"], [1941.966, 1507.476])
        self.assertEqual(gotos[len(lifecycle.APPROACH) + len(lifecycle.CULL) - 1]["xz"],
                         [2178.791, 2216.279])
        self.assertEqual(gotos[-1]["xz"], [1941.966, 1507.476])

    def test_ordinary_walk_with_room_for_the_published_weave(self):
        for row in self.gotos():
            self.assertEqual(row["radius"], 6)
            self.assertFalse(row["sprint"])
            self.assertEqual(row["timeout"], 100)

    def test_cull_leaves_the_present_ring_and_reentry_returns(self):
        stand = lifecycle.APPROACH[-1][:2]
        far = lifecycle.CULL[-1][:2]
        self.assertLess(lifecycle.hypot_xz(lifecycle.START, stand),
                        lifecycle.PRESENT_RADIUS_M)
        self.assertGreater(lifecycle.hypot_xz(lifecycle.START, far),
                           lifecycle.PRESENT_RADIUS_M)
        self.assertEqual(lifecycle.REENTRY[-1][:2], stand)

    def test_stops_before_obstruction_and_home(self):
        xs = {tuple(row["xz"]) for row in self.gotos()}
        self.assertNotIn((2160.129, 2391.969), xs)
        self.assertNotIn((2156.794, 2394.587), xs)

    def test_census_snapshots_published_path_milestones(self):
        memory = [row["line"] for row in lifecycle.route(census=True)
                  if row.get("cmd") == "console" and row.get("line") == "diagnostics memory"]
        # loaded, 200 m, 400 m, 600 m, steady, 1000 m, culled, returned
        self.assertEqual(len(memory), 8)

    def test_prepare_header_names_the_published_asset(self):
        self.assertIn("longwalk-checkpoints-r2", lifecycle.script_header())
        self.assertIn(lifecycle.LONGWALK_TRACES, lifecycle.script_header())
        self.assertNotIn("walk along +Z", lifecycle.script_header())

    def test_prepare_round_trip(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "route.jsonl"
            header = lifecycle.script_header()
            with path.open("x") as stream:
                stream.write(header)
                for row in lifecycle.route():
                    stream.write(json.dumps(row, separators=(",", ":")) + "\n")
            text = path.read_text()
            self.assertTrue(text.startswith("# Agent API 10+:"))
            commands = [json.loads(line) for line in text.splitlines() if not line.startswith("#")]
            self.assertEqual(commands[0]["xz"], list(lifecycle.AREA_1746))
            self.assertEqual(commands[-1]["cmd"], "quit")


if __name__ == "__main__":
    unittest.main()
