#!/usr/bin/env bash
# Shared local runner lifecycle. Source after setting RUNNER, PHASE and RUN_ID.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
# An explicit caller pin takes precedence over local configuration.
REQUESTED_BASE_REVISION="${ROBOT_DEMO_BASE_REVISION:-}"
# shellcheck disable=SC1091
if [ -f .env.local ]; then set -a; . ./.env.local; set +a; fi
PYTHON="${ROBOT_DEMO_PYTHON:-python3}"
PYTHON="$("$PYTHON" -c 'import sys; print(sys.executable)')"
export ROBOT_DEMO_PYTHON="$PYTHON"
BACKEND="${ROBOT_DEMO_BACKEND:-mock}"
[[ "$RUN_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || { echo "invalid run id" >&2; exit 1; }
[[ "$BACKEND" == mock || "$BACKEND" == robosuite ]] || { echo "invalid backend: $BACKEND" >&2; exit 1; }
if [ "${REQUIRE_IB:-0}" = 1 ] && ! command -v ib_console >/dev/null 2>&1; then
  echo "REQUIRE_IB=1 but ib_console is unavailable" >&2; exit 1
fi
BASE="$(git rev-parse --verify --end-of-options "${REQUESTED_BASE_REVISION:-${ROBOT_DEMO_BASE_REVISION:-HEAD}}^{commit}")"
EVIDENCE="$ROOT/evidence/$RUN_ID"
mkdir -p "$ROOT/evidence" "$ROOT/.runners"
mkdir "$EVIDENCE" || { echo "refusing to reuse evidence directory: $EVIDENCE" >&2; exit 1; }
RUNNER_DIR="$(mktemp -d "$ROOT/.runners/$RUNNER.XXXXXX")"
SRC="$RUNNER_DIR/src"
TARGET="$RUNNER_DIR/target"
export CARGO_TARGET_DIR="$TARGET"
cleanup() {
  local status=$?
  trap - EXIT
  # Export partial evidence on failures too; a failed run is never a PASS.
  if [ -d "$SRC/evidence/$RUN_ID" ]; then
    cp -R "$SRC/evidence/$RUN_ID/." "$EVIDENCE/" || status=1
  fi
  if [ -f "$SRC/.git" ]; then
    if ! git worktree remove --force "$SRC"; then
      echo "cleanup failed; inspect $RUNNER_DIR" >&2
      exit 1
    fi
  fi
  rm -rf "$RUNNER_DIR"
  if [ "$status" -ne 0 ]; then echo "runner failed; partial evidence retained at $EVIDENCE" >&2; fi
  exit "$status"
}
trap cleanup EXIT
trap 'exit 130' INT
trap 'exit 143' TERM
# This implementation always uses local worktrees; credentials do not enable a provider.
echo "== runner $RUNNER: local detached worktree @ $BASE; fresh build outputs =="
git worktree add --detach "$SRC" "$BASE" >/dev/null
mkdir "$TARGET"

ts_ms() { "$PYTHON" -c 'import time; print(time.time_ns() // 1000000)'; }
build_candidate() {
  local started finished mode
  started="$(ts_ms)"
  mode=native
  if command -v ib_console >/dev/null 2>&1; then mode=incredibuild; fi
  echo "build provider: $mode; compilation-cache reuse has not been measured"
  (
    cd "$SRC/rust"
    if [ "$mode" == incredibuild ]; then
      ib_console cargo build --locked
      ib_console cargo test -p robot-safety-gate -p swf-app --locked --no-run
    else
      cargo build --locked
      cargo test -p robot-safety-gate -p swf-app --locked --no-run
    fi
  )
  finished="$(ts_ms)"
  "$PYTHON" - "$EVIDENCE/build-metrics.jsonl" "$PHASE" "$RUNNER" "$((finished-started))" "$mode" "$BASE" <<'PY'
import json, sys
path, phase, runner, wall, provider, base = sys.argv[1:]
with open(path, "a", encoding="utf-8") as stream:
    stream.write(json.dumps({"phase": phase, "runner": runner, "wall_ms": int(wall),
        "sandbox_provider": "local_git_worktree", "build_provider": provider,
        "ib": provider == "incredibuild", "cache_namespace": None,
        "cache_reuse_verified": False, "base_revision": base}) + "\n")
PY
}

export_artifact() {
  mkdir -p "$EVIDENCE/artifact"
  cp "$TARGET/debug/swf-cli" "$EVIDENCE/artifact/"
  "$PYTHON" - "$EVIDENCE/artifact/swf-cli" <<'PY'
import hashlib, pathlib, sys
path = pathlib.Path(sys.argv[1])
path.with_suffix(".sha256").write_text(hashlib.sha256(path.read_bytes()).hexdigest() + "  swf-cli\n")
PY
}
