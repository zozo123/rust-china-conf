"""Protocol regressions; stdlib-only, with no robotics dependencies."""

import io
import json
from pathlib import Path
import sys
from types import SimpleNamespace
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bridge


class DecisionTests(unittest.TestCase):
    def decision(self, **updates):
        value = {
            "type": "decision",
            "run_id": "run",
            "episode_id": "episode",
            "action_id": "episode-approach-0",
            "simulation_tick": 0,
            "decision": "permit",
        }
        value.update(updates)
        return value

    def run_episode(self, wire):
        messages = []
        backend = bridge.MockBackend({})
        with patch.object(bridge.sys, "stdin", io.StringIO(wire)), patch.object(
            bridge, "emit", messages.append
        ):
            bridge.run_episode(
                SimpleNamespace(run_id="run", episode_id="episode"), {}, backend
            )
        return backend, messages

    def test_malformed_or_unbound_decisions_never_dispatch(self):
        missing_decision = self.decision()
        del missing_decision["decision"]
        invalid_values = [
            None,
            [],
            "permit",
            1,
            {},
            missing_decision,
            self.decision(decision="unknown"),
            self.decision(decision=None),
            self.decision(type="permit"),
            self.decision(run_id="another-run"),
            self.decision(episode_id="another-episode"),
            self.decision(action_id="another-action"),
            self.decision(simulation_tick=1),
            self.decision(simulation_tick=False),
            self.decision(simulation_tick=0.0),
            self.decision(decision="reject"),
            self.decision(decision="reject", reason=[]),
            self.decision(decision="reject", reason="unknown"),
        ]
        invalid_lines = [json.dumps(value) + "\n" for value in invalid_values]
        invalid_lines += ["not json\n", "{\n"]
        for wire in invalid_lines:
            with self.subTest(wire=wire):
                backend, messages = self.run_episode(wire)
                self.assertEqual(backend.tick, 0)
                self.assertFalse(backend.grasped)
                outcomes = [m for m in messages if m["type"] == "outcome"]
                self.assertEqual(len(outcomes), 1)
                self.assertFalse(outcomes[0]["dispatched"])
                self.assertEqual(messages[-1]["reason"], "protocol_violation")
                self.assertFalse(messages[-1]["success"])

    def test_only_explicit_bound_permits_dispatch(self):
        wire = "".join(
            json.dumps(
                self.decision(action_id=f"episode-{segment}-{tick}", simulation_tick=tick)
            )
            + "\n"
            for tick, segment in enumerate(bridge.SEGMENTS)
        )
        backend, messages = self.run_episode(wire)
        outcomes = [m for m in messages if m["type"] == "outcome"]
        self.assertEqual(backend.tick, 4)
        self.assertEqual(len(outcomes), 4)
        self.assertTrue(all(m["dispatched"] for m in outcomes))
        self.assertTrue(messages[-1]["success"])
        self.assertEqual(messages[-1]["reason"], "cube_lifted")

    def test_a_valid_rejection_ends_without_dispatch(self):
        wire = json.dumps(self.decision(decision="reject", reason="emergency_stop")) + "\n"
        backend, messages = self.run_episode(wire)
        self.assertEqual(backend.tick, 0)
        self.assertEqual(messages[-1]["reason"], "emergency_stop")
        self.assertFalse(any(m.get("dispatched") for m in messages))

    def test_old_approval_cannot_be_reused_for_the_next_action(self):
        approval = json.dumps(self.decision()) + "\n"
        backend, messages = self.run_episode(approval * 2)
        self.assertEqual(backend.tick, 1)
        self.assertEqual(sum(bool(m.get("dispatched")) for m in messages), 1)
        self.assertEqual(messages[-1]["reason"], "protocol_violation")


if __name__ == "__main__":
    unittest.main()
