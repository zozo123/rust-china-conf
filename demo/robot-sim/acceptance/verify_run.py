#!/usr/bin/env python3
"""Protected evidence verifier; candidate patches may not modify this file.

A full protected coverage matrix is required by default. Use --scenario NAME
explicitly for a single-scenario diagnostic (which is not matrix acceptance).
Expectations come from this verifier's checkout, never the evidence directory.
An artifact digest binds the recorded trace to an identified executable; it is
not independent proof that untrusted evidence was produced by that executable.
"""

import argparse
import hashlib
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
MAX_AGE_MS = 250
TICK_NS = 50_000_000


class InvalidEvidence(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidEvidence(message)


def read_json(path):
    return json.loads(path.read_text(encoding="utf-8"))


def integer(value, label):
    require(type(value) is int and value >= 0, f"{label} must be a nonnegative integer")
    return value


def protected_scenarios(root=REPO_ROOT):
    config = root / "demo/robot-sim/config"
    matrix = read_json(config / "coverage-matrix.json")
    scenarios, matrix_names = {}, []
    for path in sorted((config / "scenarios").glob("*.json")):
        scenario = read_json(path)
        name = scenario["name"]
        require(name == path.stem and name not in scenarios, "invalid protected scenario name")
        scenarios[name] = scenario
    for placement in matrix["placements"]:
        for freshness in matrix["freshness_ms"]:
            name = f"{placement['id']}-f{freshness}"
            require(name not in scenarios, f"duplicate protected scenario: {name}")
            scenarios[name] = {
                "name": name,
                "placement": {"x": placement["x"], "y": placement["y"]},
                "staleness_ms": freshness,
                "stop_at_segment": None,
                "stall_before_proposal_ms": 0,
                "expect": "rejected_stale" if freshness > MAX_AGE_MS else "cube_lifted",
            }
            matrix_names.append(name)
    matrix_names.extend(matrix["extra_scenarios"])
    require(matrix_names and len(set(matrix_names)) == len(matrix_names), "empty or duplicate protected matrix")
    require(all(name in scenarios for name in matrix_names), "unknown protected matrix scenario")
    return scenarios, matrix_names


def verify_trace(path, scenario, result, manifest):
    require(path.is_file(), f"missing trace: {path.name}")
    lines = [line for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]
    require(lines, "empty trace")
    pending = None
    seen_actions = set()
    hello = None
    terminal = None
    rejected = False
    last_dispatch_tick = None
    dispatches = decisions = holds = latest_tick = 0
    rejections = []
    run_id = manifest["run_id"]
    episode_id = f"{run_id}-{scenario}"

    for index, line in enumerate(lines, 1):
        event = json.loads(line)
        require(event["scenario"] == scenario, f"event {index}: wrong scenario")
        integer(event["ts_unix_ms"], "event timestamp")
        message = json.loads(event["line"])
        require(isinstance(message, dict), f"event {index}: message must be an object")
        require(terminal is None, "events after terminal message")
        direction = event["dir"]
        kind = message.get("type")
        if direction == "session":
            require(hello is not None, "session ended before handshake")
            require(message.get("session") == "proposal_timeout", "unexpected session failure")
            require(pending is None and decisions == 0, "timeout fixture must never authorize an action")
            terminal = {"reason": "timeout", "success": False, "ticks": latest_tick}
            continue
        require(direction == ("out" if kind == "decision" else "in"), "wrong message direction")
        if kind == "hello":
            require(hello is None and index == 1, "duplicate or late hello")
            require(type(message["version"]) is int and message["version"] == 1, "unsupported protocol version")
            require(message["simulation_tick_ns"] == TICK_NS, "wrong simulation clock")
            require(message["backend"] == manifest["simulator"]["backend"], "backend differs from manifest")
            require(message["backend_label"] == result["backend"], "backend label differs from result")
            hello = message
            continue
        require(hello is not None, "message before hello")
        if kind in ("proposal", "hold"):
            tick = integer(message["simulation_tick"], "simulation tick")
            require(tick >= latest_tick, "simulation tick moved backwards")
            require(integer(message["simulation_time_ns"], "simulation time") == tick * TICK_NS, "inconsistent simulation time")
            latest_tick = tick
        if kind == "proposal":
            require(pending is None and not rejected, "overlapping proposal or proposal after rejection")
            require(last_dispatch_tick is None or message["simulation_tick"] > last_dispatch_tick, "proposal tick did not advance after dispatch")
            require(message["run_id"] == run_id and message["episode_id"] == episode_id, "proposal run/episode mismatch")
            action = message["action_id"]
            require(isinstance(action, str) and action and action not in seen_actions, "missing or duplicate action id")
            seen_actions.add(action)
            require(type(message["simulated_stop"]) is bool, "invalid simulated stop flag")
            require(message["proposed_action"]["kind"] == "pickup", "unknown task action")
            require(all(isinstance(message["proposed_action"][key], str) and message["proposed_action"][key] for key in ("target", "segment")), "missing task target or segment")
            require(isinstance(message["observation"]["observation_id"], str) and message["observation"]["observation_id"], "missing observation identity")
            capture = integer(message["observation"]["capture_time_ns"], "capture time")
            now = message["simulation_time_ns"]
            reason, age = None, None
            if message["simulated_stop"]:
                reason = "emergency_stop"
            elif capture > now:
                reason = "invalid_timestamp"
            elif (now - capture) // 1_000_000 > MAX_AGE_MS:
                reason, age = "stale_perception", (now - capture) // 1_000_000
            pending = {"proposal": message, "decision": None, "reason": reason, "age": age}
        elif kind == "decision":
            integer(message["simulation_tick"], "decision tick")
            require(pending is not None and pending["decision"] is None, "decision without a unique proposal")
            proposal = pending["proposal"]
            require(all(message[key] == proposal[key] for key in ("run_id", "episode_id", "action_id", "simulation_tick")), "decision identity/tick mismatch")
            reason = pending["reason"]
            require(message["decision"] == ("reject" if reason else "permit"), "decision violates protected gate policy")
            if reason:
                require(message.get("reason") == reason, "incorrect rejection reason")
                suffix = reason
                if reason == "stale_perception":
                    require(message.get("age_ms") == pending["age"], "incorrect stale age")
                    suffix += f"(age={pending['age']}ms)"
                rejections.append(f"reject/{suffix}")
                rejected = True
            pending["decision"] = message
            decisions += 1
        elif kind == "outcome":
            integer(message["simulation_tick"], "outcome tick")
            require(pending is not None and pending["decision"] is not None, "outcome without prior decision")
            proposal = pending["proposal"]
            require(message["action_id"] == proposal["action_id"] and message["simulation_tick"] == proposal["simulation_tick"], "outcome action/tick mismatch")
            require(message["action_kind"] == "task" and type(message["dispatched"]) is bool, "invalid task outcome")
            require(message["dispatched"] == (pending["decision"]["decision"] == "permit"), "dispatch differs from authorization")
            dispatches += int(message["dispatched"])
            if message["dispatched"]:
                last_dispatch_tick = message["simulation_tick"]
            pending = None
        elif kind == "hold":
            require(pending is None and isinstance(message["reason"], str) and message["reason"], "unlabeled or overlapping hold")
            holds += 1
        elif kind == "episode_end":
            require(pending is None, "episode ended with incomplete action")
            require(type(message["success"]) is bool, "invalid terminal success flag")
            require(integer(message["ticks"], "terminal ticks") >= latest_tick, "terminal tick moved backwards")
            require(last_dispatch_tick is None or message["ticks"] > last_dispatch_tick, "terminal tick did not advance after dispatch")
            require(not message["success"] or (dispatches > 0 and not rejected), "success without a dispatched task or after rejection")
            terminal = message
        else:
            raise InvalidEvidence(f"unknown message type: {kind!r}")

    require(hello is not None and terminal is not None and pending is None, "missing handshake, terminal message, or outcome")
    require(result["task_dispatches"] == dispatches, "dispatch count differs from trace (holds are not dispatches)")
    require(result["decisions"] == decisions, "decision count differs from trace")
    require(result["rejections"] == rejections, "rejections differ from trace")
    require(result["outcome"] == terminal["reason"] and result["success"] == terminal["success"], "terminal result differs from trace")
    require(result["ticks"] == terminal["ticks"], "tick count differs from trace")
    return holds


def verify(evidence_dir, selected=None, root=REPO_ROOT):
    scenarios, matrix_names = protected_scenarios(root)
    names = selected if selected is not None else matrix_names
    require(names and len(set(names)) == len(names), "empty or duplicate requested scenarios")
    require(all(name in scenarios for name in names), "unknown requested scenario")
    manifest = read_json(evidence_dir / "manifest.json")
    require(manifest["run_id"] == evidence_dir.name, "manifest run id differs from evidence directory")
    require(manifest["policy_max_observation_age_ms"] == MAX_AGE_MS, "manifest changes protected freshness threshold")
    require(manifest["simulator"]["backend"] in ("mock", "robosuite"), "unknown simulator backend")
    executable = Path(manifest["executable"]["path"])
    exported = evidence_dir / "artifact" / executable.name
    if exported.is_file():
        executable = exported
    require(executable.is_file(), f"missing executable: {executable}")
    digest = hashlib.sha256(executable.read_bytes()).hexdigest()
    require(digest == manifest["executable"]["sha256"], "artifact digest differs from manifest")
    sha_file = executable.with_suffix(executable.suffix + ".sha256")
    if sha_file.exists():
        require(sha_file.read_text().split()[0] == digest, "exported checksum differs from artifact")
    results = read_json(evidence_dir / "scenario-results.json")
    require(isinstance(results, list) and results, "missing scenario results")
    actual = [result["scenario"] for result in results]
    require(len(actual) == len(set(actual)), "duplicate scenario results")
    require(set(actual) == set(names), f"scenario coverage mismatch; missing={sorted(set(names) - set(actual))}, unknown={sorted(set(actual) - set(names))}")
    traces = {path.name for path in evidence_dir.glob("events-*.jsonl")}
    require(traces == {f"events-{name}.jsonl" for name in names}, "missing or unexpected scenario traces")
    for path in (evidence_dir / "generated-scenarios").glob("*.json"):
        require(path.stem in names, f"unexpected generated scenario: {path.stem}")
        generated = read_json(path)
        require(isinstance(generated, dict), "generated scenario must be an object")
        protected = scenarios[path.stem]
        require(all(generated.get(key) == value for key, value in protected.items()), f"generated scenario differs from protected fixture: {path.stem}")
    failures = []
    for result in results:
        name = result["scenario"]
        try:
            expected = scenarios[name]["expect"]
            require(result["outcome"] == expected, f"outcome {result['outcome']!r} != expected {expected!r}")
            require(type(result["success"]) is bool and result["success"] == (expected == "cube_lifted"), "incorrect success condition")
            for key in ("task_dispatches", "decisions", "ticks", "wall_time_ms"):
                integer(result[key], key)
            require(isinstance(result["rejections"], list), "rejections must be an array")
            if expected in ("rejected_stale", "timeout"):
                require(result["task_dispatches"] == 0, "stale/timeout scenario dispatched a task action")
            if expected == "rejected_stale":
                require(any(item.startswith("reject/stale_perception(") for item in result["rejections"]), "missing stale rejection")
            if expected == "emergency_stop":
                require("reject/emergency_stop" in result["rejections"], "missing emergency-stop rejection")
            holds = verify_trace(evidence_dir / f"events-{name}.jsonl", name, result, manifest)
            print(f"  PASS  {name}: {expected}; {holds} labeled hold(s)")
        except (ValueError, KeyError, TypeError, OSError) as error:
            failures.append(f"{name}: {error}")
            print(f"  FAIL  {failures[-1]}")
    require(not failures, f"{len(failures)} scenario(s) failed verification")
    print(f"PROTECTED VERDICT: PASS ({len(names)} scenarios; {'single-scenario diagnostic' if selected else 'complete coverage matrix'})")


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("evidence_dir", type=Path)
    parser.add_argument("--scenario", action="append", help="explicit diagnostic scope; default requires the full protected matrix")
    args = parser.parse_args(argv)
    try:
        verify(args.evidence_dir.resolve(), args.scenario)
    except (ValueError, KeyError, TypeError, IndexError, OSError) as error:
        print(f"PROTECTED VERDICT: FAIL ({error})", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
