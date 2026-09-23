"""Negative tests for acceptance holes: missing coverage, forged counts and traces."""

import contextlib
import copy
import hashlib
import io
import json
import tempfile
import unittest
from pathlib import Path

import verify_run as verifier


class VerifierTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.evidence = Path(self.temp.name) / "test-run"
        self.evidence.mkdir()
        artifact = self.evidence / "artifact/swf-cli"
        artifact.parent.mkdir()
        artifact.write_bytes(b"test fixture executable")
        self.manifest = {
            "run_id": self.evidence.name,
            "policy_max_observation_age_ms": 250,
            "simulator": {"backend": "mock"},
            "executable": {"path": "/destroyed/runner/swf-cli", "sha256": hashlib.sha256(artifact.read_bytes()).hexdigest()},
        }
        self.scenarios, names = verifier.protected_scenarios()
        self.results = []
        self.traces = {}
        for name in names:
            expected = self.scenarios[name]["expect"]
            messages = [("in", {"type": "hello", "backend": "mock", "backend_label": "mock fixture", "version": 1, "simulation_tick_ns": verifier.TICK_NS})]
            rejections = []
            dispatches = decisions = tick = 0
            if expected == "timeout":
                messages.append(("session", {"session": "proposal_timeout"}))
            else:
                stale, stopped = expected == "rejected_stale", expected == "emergency_stop"
                tick = 12 if stale else 0
                identity = {"run_id": "test-run", "episode_id": f"test-run-{name}", "action_id": "action-1", "simulation_tick": tick}
                proposal = dict(identity, type="proposal", simulation_time_ns=tick * verifier.TICK_NS,
                                observation={"observation_id": "obs-0", "capture_time_ns": 0}, simulated_stop=stopped,
                                proposed_action={"kind": "pickup", "target": "cube", "segment": "approach"})
                decision = dict(identity, type="decision", decision="reject" if stale or stopped else "permit")
                if stale:
                    decision.update(reason="stale_perception", age_ms=600)
                    rejections = ["reject/stale_perception(age=600ms)"]
                elif stopped:
                    decision["reason"] = "emergency_stop"
                    rejections = ["reject/emergency_stop"]
                dispatches, decisions = int(not (stale or stopped)), 1
                messages.extend([
                    ("in", proposal), ("out", decision),
                    ("in", {"type": "outcome", "action_id": "action-1", "simulation_tick": tick, "dispatched": bool(dispatches), "action_kind": "task"}),
                    ("in", {"type": "episode_end", "reason": expected, "success": expected == "cube_lifted", "ticks": tick + dispatches}),
                ])
            self.traces[name] = [{"ts_unix_ms": 1, "dir": direction, "scenario": name, "line": json.dumps(message)} for direction, message in messages]
            self.results.append({"scenario": name, "backend": "mock fixture", "outcome": expected,
                                 "success": expected == "cube_lifted", "task_dispatches": dispatches,
                                 "decisions": decisions, "rejections": rejections, "ticks": tick + dispatches, "wall_time_ms": 1})
        self.write()

    def write(self):
        (self.evidence / "manifest.json").write_text(json.dumps(self.manifest))
        (self.evidence / "scenario-results.json").write_text(json.dumps(self.results))
        for name, trace in self.traces.items():
            (self.evidence / f"events-{name}.jsonl").write_text("".join(json.dumps(event) + "\n" for event in trace))

    def verify(self, selected=None):
        with contextlib.redirect_stdout(io.StringIO()):
            verifier.verify(self.evidence, selected)

    def rejected(self):
        self.write()
        with self.assertRaises((ValueError, KeyError, TypeError, OSError)):
            self.verify()

    def alter_message(self, name, index, **changes):
        message = json.loads(self.traces[name][index]["line"])
        message.update(changes)
        self.traces[name][index]["line"] = json.dumps(message)

    def test_accepts_complete_matrix(self):
        self.verify()

    def test_rejects_empty_results(self):
        self.results = []
        self.rejected()

    def test_rejects_missing_scenario(self):
        self.results.pop()
        self.rejected()

    def test_rejects_duplicate_results(self):
        self.results.append(copy.deepcopy(self.results[0]))
        self.rejected()

    def test_rejects_unknown_scenario(self):
        self.results[0]["scenario"] = "anything-goes"
        self.rejected()

    def test_rejects_missing_trace(self):
        (self.evidence / "events-center-f0.jsonl").unlink()
        with self.assertRaisesRegex(ValueError, "traces"):
            self.verify()

    def test_rejects_empty_trace(self):
        self.traces["center-f0"] = []
        self.rejected()

    def test_rejects_unreadable_json(self):
        self.traces["center-f0"][1]["line"] = "{"
        self.rejected()

    def test_rejects_evidence_controlled_expectation(self):
        generated = self.evidence / "generated-scenarios"
        generated.mkdir()
        scenario = dict(self.scenarios["center-f600"], expect="cube_lifted")
        (generated / "center-f600.json").write_text(json.dumps(scenario))
        self.rejected()

    def test_rejects_forged_dispatch_count(self):
        self.results[0]["task_dispatches"] += 1
        self.rejected()

    def test_rejects_boolean_count(self):
        self.results[0]["decisions"] = True
        self.rejected()

    def test_rejects_forged_rejections(self):
        self.results[0]["rejections"] = ["reject/emergency_stop"]
        self.rejected()

    def test_rejects_outcome_before_permit(self):
        events = self.traces["center-f0"]
        events[2], events[3] = events[3], events[2]
        self.rejected()

    def test_rejects_duplicate_decision(self):
        events = self.traces["center-f0"]
        events.insert(3, copy.deepcopy(events[2]))
        self.rejected()

    def test_rejects_wrong_decision_run(self):
        self.alter_message("center-f0", 2, run_id="another-run")
        self.rejected()

    def test_rejects_wrong_outcome_tick(self):
        self.alter_message("center-f0", 3, simulation_tick=99)
        self.rejected()

    def test_rejects_dispatch_after_rejection(self):
        self.alter_message("center-f600", 3, dispatched=True)
        self.rejected()

    def test_rejects_wrong_proposal_clock(self):
        self.alter_message("center-f0", 1, simulation_time_ns=123)
        self.rejected()

    def test_rejects_duplicate_hello(self):
        events = self.traces["center-f0"]
        events.insert(1, copy.deepcopy(events[0]))
        self.rejected()

    def test_rejects_success_without_any_task_dispatch(self):
        self.traces["center-f0"] = [self.traces["center-f0"][0], self.traces["center-f0"][-1]]
        self.results[0].update(task_dispatches=0, decisions=0)
        self.rejected()

    def test_rejects_terminal_tick_without_physics_advance(self):
        self.alter_message("center-f0", 4, ticks=0)
        self.results[0]["ticks"] = 0
        self.rejected()

    def test_rejects_missing_terminal(self):
        self.traces["center-f0"].pop()
        self.rejected()

    def test_rejects_event_after_terminal(self):
        self.traces["center-f0"].append(copy.deepcopy(self.traces["center-f0"][1]))
        self.rejected()

    def test_rejects_missing_outcome(self):
        self.traces["center-f0"].pop(3)
        self.rejected()

    def test_rejects_terminal_result_mismatch(self):
        self.alter_message("center-f0", 4, success=False)
        self.rejected()

    def test_rejects_invalid_backend(self):
        self.manifest["simulator"]["backend"] = "real-hardware"
        self.rejected()

    def test_rejects_changed_artifact(self):
        (self.evidence / "artifact/swf-cli").write_bytes(b"changed")
        self.rejected()

    def test_rejects_changed_threshold(self):
        self.manifest["policy_max_observation_age_ms"] = 999
        self.rejected()

    def test_explicit_diagnostic_scope_does_not_claim_matrix(self):
        keep = "center-f0"
        self.results = [result for result in self.results if result["scenario"] == keep]
        self.traces = {keep: self.traces[keep]}
        for path in self.evidence.glob("events-*.jsonl"):
            path.unlink()
        self.write()
        self.verify([keep])
        with self.assertRaisesRegex(ValueError, "coverage"):
            self.verify()

    def test_cli_fails_cleanly_on_missing_manifest(self):
        (self.evidence / "manifest.json").unlink()
        with contextlib.redirect_stderr(io.StringIO()):
            self.assertEqual(verifier.main([str(self.evidence)]), 1)


if __name__ == "__main__":
    unittest.main()
