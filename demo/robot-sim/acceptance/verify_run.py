#!/usr/bin/env python3
"""PROTECTED ACCEPTANCE FIXTURE — DO NOT EDIT.

The agent work order for the seeded-regression exercise forbids modifying
this verifier. The factory's diff validator rejects candidate patches that
touch acceptance paths, and the verifier runs from a protected copy, not
from anything the agent reports.

Verifies an evidence directory (evidence/<run-id>/):
  * trace linkage: every dispatch is backed by a permit for the exact
    action id + simulation tick; every rejection is followed by no dispatch
  * scenario expectations from the scenario configs (expect field)
  * artifact identity: the recorded executable digest matches the file
  * hold steps are labeled and never counted as task dispatches

Exit code 0 = PASS, 1 = FAIL.
"""

import hashlib
import json
import sys
from pathlib import Path

FAILURES = []
CHECKS = 0


def check(ok: bool, label: str) -> None:
    global CHECKS
    CHECKS += 1
    print(f"  {'PASS' if ok else 'FAIL'}  {label}")
    if not ok:
        FAILURES.append(label)


def load_jsonl(path: Path):
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text().splitlines() if line.strip()]


def scenario_expectations(evidence_dir: Path) -> dict:
    """Expectations: from generated matrix scenarios and shipped configs."""
    expect = {}
    gen = evidence_dir / "generated-scenarios"
    if gen.is_dir():
        for f in gen.glob("*.json"):
            s = json.loads(f.read_text())
            expect[s["name"]] = s.get("expect")
    repo_root = evidence_dir.parent.parent
    shipped = repo_root / "demo" / "robot-sim" / "config" / "scenarios"
    if shipped.is_dir():
        for f in shipped.glob("*.json"):
            s = json.loads(f.read_text())
            expect.setdefault(s["name"], s.get("expect"))
    return expect


def verify_trace(events_path: Path, scenario: str) -> None:
    events = load_jsonl(events_path)
    proposals, decisions, outcomes, holds = {}, {}, [], 0
    for e in events:
        try:
            msg = json.loads(e["line"])
        except json.JSONDecodeError:
            continue
        t = msg.get("type")
        if t == "proposal":
            proposals[msg["action_id"]] = msg
        elif t == "decision":
            decisions[msg["action_id"]] = msg
        elif t == "outcome":
            outcomes.append(msg)
        elif t == "hold":
            holds += 1

    for o in outcomes:
        aid = o["action_id"]
        if not o.get("dispatched"):
            continue
        d = decisions.get(aid)
        p = proposals.get(aid)
        check(
            d is not None
            and d.get("decision") == "permit"
            and p is not None
            and d.get("simulation_tick") == p.get("simulation_tick"),
            f"[{scenario}] dispatch {aid} bound to exact permit (action id + tick)",
        )

    for aid, d in decisions.items():
        if d.get("decision") != "reject":
            continue
        dispatched = any(
            o["action_id"] == aid and o.get("dispatched") for o in outcomes
        )
        check(
            not dispatched,
            f"[{scenario}] rejection {aid} ({d.get('reason')}) produced no dispatch",
        )
    print(f"  info  [{scenario}] {holds} labeled hold step(s)")


def main() -> int:
    evidence_dir = Path(sys.argv[1]).resolve()
    print(f"protected verifier: {evidence_dir}")

    manifest = json.loads((evidence_dir / "manifest.json").read_text())
    results = json.loads((evidence_dir / "scenario-results.json").read_text())
    expect = scenario_expectations(evidence_dir)

    # Artifact identity: digest in the manifest must match the executable.
    exe_path = Path(manifest["executable"]["path"])
    if exe_path.exists():
        digest = hashlib.sha256(exe_path.read_bytes()).hexdigest()
        check(
            digest == manifest["executable"]["sha256"],
            "artifact identity: executable digest matches manifest",
        )
    else:
        check(False, f"artifact identity: executable missing at {exe_path}")

    by_scenario = {}
    for r in results:
        by_scenario.setdefault(r["scenario"], []).append(r)

    for scenario, runs in by_scenario.items():
        r = runs[-1]  # latest result for the scenario
        expected = expect.get(scenario)
        check(
            expected is None or r["outcome"] == expected,
            f"[{scenario}] outcome {r['outcome']!r} == expected {expected!r}",
        )
        if expected == "rejected_stale":
            check(
                r["task_dispatches"] == 0,
                f"[{scenario}] zero task dispatches on stale episode",
            )
            check(
                any("stale_perception" in x for x in r["rejections"]),
                f"[{scenario}] StalePerception rejection recorded",
            )
        if expected == "cube_lifted":
            check(r["success"], f"[{scenario}] cube-lift success condition reached")
        if expected == "emergency_stop":
            check(
                any("emergency_stop" in x for x in r["rejections"]),
                f"[{scenario}] EmergencyStop rejection recorded",
            )
        if expected == "timeout":
            check(
                r["outcome"] == "timeout" and r["task_dispatches"] == 0,
                f"[{scenario}] explicit timeout failure, no dispatch",
            )
        verify_trace(evidence_dir / f"events-{scenario}.jsonl", scenario)

    print(f"\nPROTECTED VERDICT: {'PASS' if not FAILURES else 'FAIL'} "
          f"({CHECKS - len(FAILURES)}/{CHECKS} checks)")
    return 0 if not FAILURES else 1


if __name__ == "__main__":
    sys.exit(main())
