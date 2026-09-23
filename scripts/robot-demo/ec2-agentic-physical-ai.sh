#!/usr/bin/env bash
# Conference-grade EC2-hosted agentic physical-AI demonstration.
#
# "Sandbox" here means a disposable git worktree + fresh CARGO_TARGET_DIR on
# the EC2 Initiator. This script does not claim to provision or destroy EC2.
# The robot is robosuite/MuJoCo SIL; no physical hardware is controlled.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
RUN_ID="${1:-ec2-agentic-$(date +%s)}"
TIMELINE="$ROOT/evidence/$RUN_ID-e2e"
[[ ! -e "$TIMELINE" ]] || { echo "refusing to reuse $TIMELINE" >&2; exit 2; }
mkdir -p "$TIMELINE"

export REQUIRE_IB=1
export ROBOT_DEMO_BACKEND=robosuite

ts_ms() { python3 -c 'import time; print(time.time_ns() // 1000000)'; }
record_phase() {
  python3 - "$TIMELINE/timeline.jsonl" "$1" "$2" "$3" <<'PY'
import json, sys
path, phase, start, finish = sys.argv[1:]
with open(path, "a", encoding="utf-8") as stream:
    stream.write(json.dumps({
        "phase": phase,
        "start_unix_ms": int(start),
        "finish_unix_ms": int(finish),
        "wall_ms": int(finish) - int(start),
    }, sort_keys=True) + "\n")
PY
}

echo "== preflight: EC2 Initiator + IB + real SIL =="
scripts/robot-demo/preflight.sh

echo "== controlled build experiment: same candidate, >=5 samples per mode =="
start="$(ts_ms)"
scripts/robot-demo/ib-benchmark.sh "$RUN_ID-build"
finish="$(ts_ms)"
record_phase build_experiment "$start" "$finish"

echo "== agentic behavior loop: seed -> bounded patch -> fresh workspace -> proof =="
start="$(ts_ms)"
scripts/robot-demo/rehearse.sh "$RUN_ID-behavior"
finish="$(ts_ms)"
record_phase behavior_rehearsal "$start" "$finish"

python3 - "$TIMELINE/e2e-receipt.json" "$RUN_ID" <<'PY'
import json, pathlib, sys
path, run_id = map(pathlib.Path, [sys.argv[1], sys.argv[2]])
root = path.parent.parent
timeline = [
    json.loads(line)
    for line in (path.parent / "timeline.jsonl").read_text().splitlines()
]
receipt = {
    "schema_version": 1,
    "run_id": str(run_id),
    "execution_host": "EC2 Incredibuild Initiator",
    "sandbox_semantics": "disposable git worktrees and fresh Cargo target directories",
    "build_proof": str(root / f"{run_id}-build" / "build-proof" / "receipt.json"),
    "seeded_failure": str(root / f"{run_id}-behavior-runner-a"),
    "validated_candidate": str(root / f"{run_id}-behavior-runner-b"),
    "timeline": timeline,
    "scope": {
        "performed": [
            "Rust native/IB/cache build experiment",
            "Incredibuild remote-task verification",
            "parent-warmed local-user cache verification",
            "robosuite/MuJoCo software-in-the-loop scenarios",
            "digest-bound protected verification",
        ],
        "not_performed": [
            "EC2 instance provisioning or destruction",
            "hardware-in-the-loop",
            "physical robot validation",
            "trained vision",
        ],
    },
}
path.write_text(json.dumps(receipt, indent=2) + "\n")
PY

cat <<EOF

E2E complete: $RUN_ID
  build proof:        evidence/$RUN_ID-build/build-proof/receipt.json
  seeded failure:     evidence/$RUN_ID-behavior-runner-a
  validated behavior: evidence/$RUN_ID-behavior-runner-b
  timing receipt:     evidence/$RUN_ID-e2e/e2e-receipt.json

Claim boundary:
  EC2-hosted disposable workspaces, not provisioned/destroyed EC2 instances.
  Software-in-the-loop, not hardware-in-the-loop.
EOF
