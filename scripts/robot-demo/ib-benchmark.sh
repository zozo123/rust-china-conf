#!/usr/bin/env bash
# Controlled proof of Rust distribution and parent-warmed Build Cache reuse.
#
# Run this on the EC2 Incredibuild Initiator, from a clean repository clone.
# It deliberately clears the current user's IB local cache before each cold
# and parent-seed pair. It never clears a shared/service cache.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
RUN_ID="${1:-ib-proof-$(date +%s)}"
SAMPLES="${IB_SAMPLES:-5}"
PATCH="${ROBOT_DEMO_PATCH_FILE:-$ROOT/demo/fallback-patch.diff}"
BASE="${ROBOT_DEMO_BASE_REVISION:-$(git rev-parse HEAD)}"
OUT="$ROOT/evidence/$RUN_ID/build-proof"
RAW="$OUT/raw"
SAMPLE_LOG="$OUT/samples.jsonl"
PROOF="$OUT/receipt.json"
IB_CACHE_TOOL="/opt/incredibuild/management/build_avoid_cache.sh"
IB_STATS_TOOL="/opt/incredibuild/management/show_build_cache_statistics.sh"
PROOF_CLI="$ROOT/rust/target/debug/swf-cli"

[[ "$RUN_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || {
  echo "invalid run id" >&2; exit 2;
}
[[ "$SAMPLES" =~ ^[0-9]+$ && "$SAMPLES" -ge 5 ]] || {
  echo "IB_SAMPLES must be at least 5" >&2; exit 2;
}
[[ "${IB_ALLOW_CLEAR_USER_CACHE:-0}" == 1 ]] || {
  echo "set IB_ALLOW_CLEAR_USER_CACHE=1: this benchmark clears only qa_user's local IB cache" >&2
  exit 2
}
[[ -n "${IB_HISTORY_URL:-}" && -n "${IB_CLIENT_API_KEY:-}" ]] || {
  echo "IB_HISTORY_URL and IB_CLIENT_API_KEY are required for remote-task proof" >&2
  exit 2
}
command -v ib_console >/dev/null
command -v curl >/dev/null
[[ -x "$IB_CACHE_TOOL" && -x "$IB_STATS_TOOL" ]] || {
  echo "Incredibuild cache management tools are unavailable" >&2; exit 2;
}
[[ ! -e "$OUT" ]] || { echo "refusing to reuse $OUT" >&2; exit 2; }
mkdir -p "$RAW"
: > "$SAMPLE_LOG"

# Build the Rust evidence tool before the measured experiment. Each benchmark
# build uses a separate fresh target directory, so this bootstrap cannot warm it.
cargo build --manifest-path "$ROOT/rust/Cargo.toml" --locked -q -p swf-cli

WORK="$(mktemp -d "$ROOT/.ib-proof.XXXXXX")"
PARENT_SRC="$WORK/parent"
CANDIDATE_SRC="$WORK/candidate"
cleanup() {
  local status=$?
  git worktree remove --force "$PARENT_SRC" >/dev/null 2>&1 || true
  git worktree remove --force "$CANDIDATE_SRC" >/dev/null 2>&1 || true
  rm -rf "$WORK"
  exit "$status"
}
trap cleanup EXIT INT TERM

git worktree add --detach "$PARENT_SRC" "$BASE" >/dev/null
git worktree add --detach "$CANDIDATE_SRC" "$BASE" >/dev/null
python3 "$ROOT/scripts/robot-demo/check-patch.py" "$PATCH"
git -C "$CANDIDATE_SRC" apply --check -- "$PATCH"
git -C "$CANDIDATE_SRC" apply -- "$PATCH"
PATCH_SHA="$(sha256sum "$PATCH" | awk '{print $1}')"
CANDIDATE_ID="${BASE}+patch:${PATCH_SHA}"

# Downloads and index updates are outside every timed interval.
(cd "$PARENT_SRC/rust" && cargo fetch --locked)
(cd "$CANDIDATE_SRC/rust" && cargo fetch --locked)

ts_ms() { date +%s%3N; }

history_snapshot() {
  local output="$1"
  local insecure=()
  if [[ "${IB_HISTORY_CURL_INSECURE:-0}" == 1 ]]; then insecure=(-k); fi
  curl --fail --silent --show-error "${insecure[@]}" \
    -H "client-api-key: $IB_CLIENT_API_KEY" "$IB_HISTORY_URL" > "$output"
}

wait_for_history() {
  local caption="$1" output="$2"
  for _ in $(seq 1 12); do
    history_snapshot "$output"
    if "$PROOF_CLI" robot-demo build-history \
        --input "$output" --caption "$caption" >/dev/null 2>&1; then
      return
    fi
    sleep 2
  done
  echo "Build History API never returned exactly one record for $caption" >&2
  return 1
}

cache_stats() {
  local history="$1" caption="$2" output="$3"
  local build_number
  build_number="$("$PROOF_CLI" robot-demo build-history \
    --input "$history" --caption "$caption" --build-number-only)"
  "$IB_STATS_TOOL" "$build_number" > "$output"
  # Fail now if this version's output cannot be parsed; never record inferred data.
  "$PROOF_CLI" robot-demo build-sample \
    --mode ib-cold --repetition 1 --wall-ms 1 --caption "$caption" \
    --source-revision parser-check --history "$history" --cache "$output" >/dev/null
}

fresh_target() {
  mktemp -d "$WORK/target.XXXXXX"
}

native_sample() {
  local rep="$1" target start finish caption
  caption="$RUN_ID-native-$rep"
  target="$(fresh_target)"
  start="$(ts_ms)"
  (cd "$CANDIDATE_SRC/rust" && CARGO_TARGET_DIR="$target" cargo build --workspace --locked)
  finish="$(ts_ms)"
  "$PROOF_CLI" robot-demo build-sample \
    --mode native --repetition "$rep" --wall-ms "$((finish-start))" \
    --caption "$caption" --source-revision "$CANDIDATE_ID" >> "$SAMPLE_LOG"
  rm -rf "$target"
}

ib_build() {
  local source="$1" target="$2" caption="$3" log="$4"
  (
    cd "$source/rust"
    CARGO_TARGET_DIR="$target" ib_console \
      -c "$caption" -f \
      --build-cache-local-user \
      --build-cache-report-all-miss \
      cargo build --workspace --locked
  ) 2>&1 | tee "$log"
}

ib_sample() {
  local mode="$1" rep="$2" source="$3"
  local target caption log history cache start finish
  caption="$RUN_ID-$mode-$rep"
  target="$(fresh_target)"
  log="$RAW/$caption.console.txt"
  history="$RAW/$caption.history.json"
  cache="$RAW/$caption.cache.txt"
  start="$(ts_ms)"
  ib_build "$source" "$target" "$caption" "$log"
  finish="$(ts_ms)"
  wait_for_history "$caption" "$history"
  cache_stats "$history" "$caption" "$cache"
  "$PROOF_CLI" robot-demo build-sample \
    --mode "$mode" --repetition "$rep" --wall-ms "$((finish-start))" \
    --caption "$caption" --source-revision "$CANDIDATE_ID" \
    --history "$history" --cache "$cache" >> "$SAMPLE_LOG"
  rm -rf "$target"
}

seed_parent_cache() {
  local rep="$1" target caption log history cache
  caption="$RUN_ID-parent-seed-$rep"
  target="$(fresh_target)"
  log="$RAW/$caption.console.txt"
  history="$RAW/$caption.history.json"
  cache="$RAW/$caption.cache.txt"
  ib_build "$PARENT_SRC" "$target" "$caption" "$log"
  wait_for_history "$caption" "$history"
  cache_stats "$history" "$caption" "$cache"
  rm -rf "$target"
}

{
  echo "run_id=$RUN_ID"
  echo "base_revision=$BASE"
  echo "candidate_identity=$CANDIDATE_ID"
  echo "samples_per_mode=$SAMPLES"
  echo "timed_command=cargo build --workspace --locked"
  echo "downloads_outside_timing=true"
  echo "cache_scope=local-user"
  echo "run_order=rotating native/ib-cold/ib-parent-warm"
} > "$OUT/method.txt"

for rep in $(seq 1 "$SAMPLES"); do
  echo "== repetition $rep / $SAMPLES =="
  case $((rep % 3)) in
    1) order=(native ib-cold ib-parent-warm) ;;
    2) order=(ib-cold ib-parent-warm native) ;;
    0) order=(ib-parent-warm native ib-cold) ;;
  esac
  printf '%s %s\n' "$rep" "${order[*]}" >> "$OUT/run-order.txt"
  for mode in "${order[@]}"; do
    case "$mode" in
      native)
        native_sample "$rep"
        ;;
      ib-cold)
        "$IB_CACHE_TOOL" user clear
        ib_sample ib-cold "$rep" "$CANDIDATE_SRC"
        ;;
      ib-parent-warm)
        "$IB_CACHE_TOOL" user clear
        seed_parent_cache "$rep"
        ib_sample ib-parent-warm "$rep" "$CANDIDATE_SRC"
        ;;
    esac
  done
done

"$PROOF_CLI" robot-demo build-receipt \
  --samples "$SAMPLE_LOG" --run-id "$RUN_ID" \
  --candidate-revision "$CANDIDATE_ID" --parent-revision "$BASE" \
  --output "$PROOF"

# The Rust verifier is the final gate and computes only measured ratios.
"$PROOF_CLI" robot-demo build-proof --receipt "$PROOF" --min-samples "$SAMPLES" |
  tee "$OUT/summary.txt"

echo "verified build proof: $PROOF"
