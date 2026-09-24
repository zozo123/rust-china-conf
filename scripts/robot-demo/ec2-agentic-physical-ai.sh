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
# Each stage runs under its own derived run id, and every path this script
# reports or records is derived here, once, from the same variables the stages
# are launched with. Nothing downstream reconstructs a path by hand.
BUILD_RUN_ID="$RUN_ID-build"
BEHAVIOR_RUN_ID="$RUN_ID-behavior"
TIMELINE="$ROOT/evidence/$RUN_ID-e2e"
BUILD_PROOF="$ROOT/evidence/$BUILD_RUN_ID/build-proof/receipt.json"
BUILD_PROOF_SUMMARY="$ROOT/evidence/$BUILD_RUN_ID/build-proof/summary.txt"
SEEDED_FAILURE="$ROOT/evidence/$BEHAVIOR_RUN_ID-runner-a"
VALIDATED_CANDIDATE="$ROOT/evidence/$BEHAVIOR_RUN_ID-runner-b"
[[ ! -e "$TIMELINE" ]] || { echo "refusing to reuse $TIMELINE" >&2; exit 2; }
mkdir -p "$TIMELINE"

# A missing artifact is reported with the path that was actually expected, and
# stops the run. It is never passed on to the receipt as an unchecked string.
require_path() {
  local kind="$1" path="$2" produced_by="$3"
  if [ ! -e "$path" ]; then
    echo "FAIL expected $kind at:" >&2
    echo "       $path" >&2
    echo "     produced by: $produced_by" >&2
    echo "     present under $(dirname "$path"):" >&2
    # A human-readable listing of what is actually there; not machine-parsed.
    # shellcheck disable=SC2012
    ls -1 "$(dirname "$path")" 2>&1 | sed 's/^/       /' >&2
    exit 1
  fi
  echo "ok   $kind: $path"
}

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
scripts/robot-demo/ib-benchmark.sh "$BUILD_RUN_ID"
finish="$(ts_ms)"
record_phase build_experiment "$start" "$finish"
require_path "build-proof receipt" "$BUILD_PROOF" "scripts/robot-demo/ib-benchmark.sh $BUILD_RUN_ID"
require_path "build-proof summary" "$BUILD_PROOF_SUMMARY" "scripts/robot-demo/ib-benchmark.sh $BUILD_RUN_ID"

echo "== agentic behavior loop: seed -> bounded patch -> fresh workspace -> proof =="
start="$(ts_ms)"
scripts/robot-demo/rehearse.sh "$BEHAVIOR_RUN_ID"
finish="$(ts_ms)"
record_phase behavior_rehearsal "$start" "$finish"
require_path "seeded failure evidence" "$SEEDED_FAILURE" "scripts/robot-demo/rehearse.sh $BEHAVIOR_RUN_ID"
require_path "validated candidate evidence" "$VALIDATED_CANDIDATE" "scripts/robot-demo/rehearse.sh $BEHAVIOR_RUN_ID"

# The paths below were checked above; they are passed in rather than rebuilt.
python3 - "$TIMELINE/e2e-receipt.json" "$RUN_ID" \
  "$BUILD_PROOF" "$SEEDED_FAILURE" "$VALIDATED_CANDIDATE" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
run_id = sys.argv[2]
build_proof, seeded_failure, validated_candidate = sys.argv[3:6]
missing = [p for p in (build_proof, seeded_failure, validated_candidate)
           if not pathlib.Path(p).exists()]
if missing:
    raise SystemExit("refusing to write a receipt that points at missing evidence: "
                     + ", ".join(missing))
timeline = [
    json.loads(line)
    for line in (path.parent / "timeline.jsonl").read_text().splitlines()
]
receipt = {
    "schema_version": 1,
    "run_id": run_id,
    "execution_host": "EC2 Incredibuild Initiator",
    "sandbox_semantics": "disposable git worktrees and fresh Cargo target directories",
    "build_proof": build_proof,
    "seeded_failure": seeded_failure,
    "validated_candidate": validated_candidate,
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
  build proof:        $BUILD_PROOF
  seeded failure:     $SEEDED_FAILURE
  validated behavior: $VALIDATED_CANDIDATE
  timing receipt:     $TIMELINE/e2e-receipt.json

Claim boundary:
  EC2-hosted disposable workspaces, not provisioned/destroyed EC2 instances.
  Software-in-the-loop, not hardware-in-the-loop.
EOF
