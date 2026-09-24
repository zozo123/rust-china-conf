#!/usr/bin/env bash
# Conference-grade EC2-hosted agentic physical-AI demonstration.
#
# "Sandbox" here means a disposable git worktree plus a disposable Cargo
# workspace on the EC2 Initiator: the target directory's PATH is held stable
# and its CONTENTS are wiped before every build. That is deliberate, not an
# oversight. Incredibuild's Build Cache key includes rustc's --out-dir, so a
# freshly-mktemp'd target directory (what this demo used to do) guarantees ~0%
# reuse by construction; holding the path and wiping the contents is what lets
# "disposable workspace, reusable compilation" be measured rather than asserted.
# See the WORKSPACE DISCIPLINE block in scripts/robot-demo/ib-benchmark.sh.
#
# The build stage below defaults to CACHE-ONLY acceleration: ib-benchmark.sh
# installs rust/ib_profile.cache-only.xml, which declares rustc local_only with
# ib_cache enabled, so nothing is sent to a helper and the verifier is run with
# --distribution excluded, which refuses the receipt if anything was. Export
# IB_ACCEL=distributed to run the allow_remote variant instead; the mode, the
# profile digest and the contract are recorded in the build stage's method.txt
# either way, and repeated in this script's own receipt below.
#
# This script does not claim to provision or destroy EC2.
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
# Recorded in the receipt below, so the e2e artifact says which acceleration
# the build stage measured rather than leaving a reader to guess.
IB_ACCEL="${IB_ACCEL:-cache-only}"
export IB_ACCEL
case "$IB_ACCEL" in
  cache-only|distributed) ;;
  *) echo "IB_ACCEL must be cache-only or distributed (got '$IB_ACCEL')" >&2; exit 2 ;;
esac
# A smoke rehearsal writes no receipt, so the e2e would fail at require_path
# with a confusing message about a missing file. Refuse it up front instead.
if [[ "${IB_SMOKE:-0}" == 1 ]]; then
  echo "IB_SMOKE=1 makes ib-benchmark.sh skip build-receipt, so this end-to-end run" >&2
  echo "would have no build proof to require. Run the smoke rehearsal on its own." >&2
  exit 2
fi

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
  "$BUILD_PROOF" "$SEEDED_FAILURE" "$VALIDATED_CANDIDATE" "$IB_ACCEL" <<'PY'
import json, pathlib, sys
path = pathlib.Path(sys.argv[1])
run_id = sys.argv[2]
build_proof, seeded_failure, validated_candidate, ib_accel = sys.argv[3:7]
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
    "sandbox_semantics": ("disposable git worktrees; Cargo target directories whose path is "
                          "held stable and whose contents are wiped before every build, so that "
                          "cargo has no incremental state and only the Incredibuild Build Cache "
                          "can serve a compilation"),
    "acceleration_mode": ib_accel,
    "acceleration_note": (
        "cache-only installs rust/ib_profile.cache-only.xml (rustc local_only with "
        "ib_cache enabled) and verifies the receipt with --distribution excluded, so "
        "Incredibuild's own Build History must report zero remote tasks and zero remote "
        "core time for every measured build and every parent seed; distributed installs "
        "rust/ib_profile.xml (rustc allow_remote) and requires the opposite. The profile "
        "digest actually installed is in the build stage's method.txt and profile-check.txt."
        if ib_accel in ("cache-only", "distributed") else "unrecognized"
    ),
    "build_proof": build_proof,
    "seeded_failure": seeded_failure,
    "validated_candidate": validated_candidate,
    "timeline": timeline,
    "scope": {
        "performed": [
            "Rust native/IB/cache build experiment",
            ("Incredibuild remote-task verification: zero remote tasks REQUIRED and checked"
             if ib_accel == "cache-only" else
             "Incredibuild remote-task verification: remote tasks required and checked"),
            "parent-warmed local-user Build Cache reuse, measured from Incredibuild's "
            "own per-task cache report and build report DB counters",
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
  acceleration:       $IB_ACCEL
  build proof:        $BUILD_PROOF
  seeded failure:     $SEEDED_FAILURE
  validated behavior: $VALIDATED_CANDIDATE
  timing receipt:     $TIMELINE/e2e-receipt.json

Claim boundary:
  EC2-hosted disposable workspaces, not provisioned/destroyed EC2 instances.
  Software-in-the-loop, not hardware-in-the-loop.
EOF
