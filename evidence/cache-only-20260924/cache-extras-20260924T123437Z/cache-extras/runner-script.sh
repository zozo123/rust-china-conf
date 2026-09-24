#!/usr/bin/env bash
# PHASE 2 -- side measurements the gated benchmark does not cover.
#
# Modes measured here, 5 samples each, rotating order:
#   native              plain cargo, fresh (wiped) target dir            [anchor, repeats phase 1]
#   native-debug0       plain cargo, CARGO_PROFILE_DEV_DEBUG=0           [Rust lever: no debuginfo]
#   native-lld          plain cargo, rust-lld linker                     [Rust lever: faster linker]
#   ib-warm-identical   Incredibuild CACHE-ONLY, cache warmed by an IDENTICAL build at the
#                       SAME target path, target contents wiped before every sample.
#                       This is the full-reuse "disposable workspace" configuration.
#
# NOT gated by swf-cli build-proof: the receipt schema knows three modes only.
# Every cache counter here still comes from Incredibuild's own telemetry
# (show_build_cache_statistics.sh over the build report DB) cross-checked
# against the per-task Build Cache report, and every remote-task counter comes
# from the Build History API. Nothing is inferred from wall time.
set -euo pipefail
ROOT="$HOME/rust-china-conf"
cd "$ROOT"
. "$ROOT/scripts/robot-demo/env-local.sh"

RUN_ID="${1:?run id required}"
SAMPLES="${IB_SAMPLES:-5}"
BASE="$(git rev-parse HEAD)"
OUT="$ROOT/evidence/$RUN_ID/cache-extras"
RAW="$OUT/raw"
LOG="$OUT/samples.jsonl"
IB_CACHE_TOOL="/opt/incredibuild/management/build_avoid_cache.sh"
IB_STATS_TOOL="/opt/incredibuild/management/show_build_cache_statistics.sh"
CACHE_CLEAR="$ROOT/scripts/robot-demo/cache-clear.sh"
PROOF_CLI="$ROOT/rust/target/debug/swf-cli"
IB_PROFILE="$ROOT/rust/ib_profile.cache-only.xml"

[[ -x "$PROOF_CLI" ]] || { echo "missing $PROOF_CLI" >&2; exit 2; }
[[ -n "${IB_HISTORY_URL:-}" && -n "${IB_CLIENT_API_KEY:-}" ]] || { echo "history creds required" >&2; exit 2; }
[[ "${IB_ALLOW_CLEAR_USER_CACHE:-0}" == 1 ]] || { echo "consent required" >&2; exit 2; }
[[ ! -e "$OUT" ]] || { echo "refusing to reuse $OUT" >&2; exit 2; }
[[ "$SAMPLES" -ge 5 ]] || { echo "need >=5 samples" >&2; exit 2; }
mkdir -p "$RAW"; : > "$LOG"

TOOLCHAIN_BIN="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/bin"
LLD_FLAGS="-Clink-arg=-fuse-ld=lld -Clink-arg=-B$TOOLCHAIN_BIN/gcc-ld"
[[ -x "$TOOLCHAIN_BIN/gcc-ld/ld.lld" ]] || { echo "no rust-lld shim at $TOOLCHAIN_BIN/gcc-ld/ld.lld" >&2; exit 2; }

WORK="$(mktemp -d "$ROOT/.ib-extras.XXXXXX")"
SRC="$WORK/src"
cleanup(){ local s=$?; git worktree remove --force "$SRC" >/dev/null 2>&1 || true; rm -rf "$WORK"; exit $s; }
trap cleanup EXIT INT TERM
git worktree add --detach "$SRC" "$BASE" >/dev/null
cp "$IB_PROFILE" "$SRC/rust/ib_profile.xml"
PROFILE_SHA="$(sha256sum "$SRC/rust/ib_profile.xml" | awk '{print $1}')"
(cd "$SRC/rust" && cargo fetch --locked)          # outside every timed phase

WS="$WORK/ws"; mkdir -p "$WS/native" "$WS/native-debug0" "$WS/native-lld" "$WS/ib"
ts_ms(){ date +%s%3N; }
wipe(){ rm -rf "$1"; mkdir -p "$1"; printf '%s\n' "$1"; }

history_snapshot(){ local o="$1" k=(); [[ "${IB_HISTORY_CURL_INSECURE:-0}" == 1 ]] && k=(-k)
  curl --fail --silent --show-error "${k[@]}" -H "client-api-key: $IB_CLIENT_API_KEY" "$IB_HISTORY_URL" > "$o"; }
wait_for_history(){ local cap="$1" o="$2"; for _ in $(seq 1 12); do history_snapshot "$o"
    "$PROOF_CLI" robot-demo build-history --input "$o" --caption "$cap" >/dev/null 2>&1 && return 0; sleep 2; done
  echo "no unique Build History record for $cap" >&2; return 1; }

IB_CONSOLE_ARGS=(--build-cache-local-user --build-cache-report-all-miss)

ib_build(){ local cap="$1" target="$2" log="$3"
  printf '%s\n' "ib_console -c $cap ${IB_CONSOLE_ARGS[*]} cargo build --workspace --locked" > "$RAW/$cap.ib-argv.txt"
  ( cd "$SRC/rust" && CARGO_TARGET_DIR="$target" ib_console -c "$cap" "${IB_CONSOLE_ARGS[@]}" \
      cargo build --workspace --locked ) 2>&1 | tee "$log" >/dev/null; }

# Returns "total hits misses remote remote_core_time"; aborts if the per-task
# Build Cache report disagrees with the build report DB aggregate.
ib_counters(){ local cap="$1" log="$2"
  local hist="$RAW/$cap.history.json" cache="$RAW/$cap.cache.txt" bn rep src dh agg
  wait_for_history "$cap" "$hist"
  bn="$("$PROOF_CLI" robot-demo build-history --input "$hist" --caption "$cap" --build-number-only)"
  "$IB_STATS_TOOL" "$bn" > "$cache"
  src="$(sed -n "s/.*Build Cache report is '\([^']*\)'.*/\1/p" "$log" | tail -n 1)"
  [[ -n "$src" && -r "$src" ]] || { echo "$cap: no Build Cache report path" >&2; return 1; }
  rep="$RAW/$cap.cache-report.txt"; cp "$src" "$rep"
  dh="$(grep -c '^HIT:' "$rep" || true)"
  agg="$(sed -n 's/^hits=//p' "$cache")"
  [[ "$dh" == "$agg" ]] || { echo "$cap: per-task report $dh hits vs DB $agg" >&2; return 1; }
  python3 - "$cache" "$hist" "$cap" "$PROOF_CLI" <<'PY'
import json,subprocess,sys,re
cache,hist,cap,cli=sys.argv[1:5]
k=dict(re.findall(r'^(\w+)=(\S+)$',open(cache).read(),re.M))
h=json.loads(subprocess.run([cli,"robot-demo","build-history","--input",hist,"--caption",cap],
    capture_output=True,text=True,check=True).stdout)
print(k["total"],k["hits"],k["misses"],h["remote_tasks"],h["remote_core_time_s"],h["build_number"])
PY
}

emit(){ python3 -c '
import json,sys
mode,rep,wall,cap,tot,hits,miss,rt,rct,bn,note=sys.argv[1:12]
num=lambda v: None if v=="-" else (float(v) if "." in v else int(v))
print(json.dumps({"mode":mode,"repetition":int(rep),"wall_ms":int(wall),"build_caption":cap,
 "cache_total":num(tot),"cache_hits":num(hits),"cache_misses":num(miss),
 "remote_tasks":num(rt),"remote_core_time_s":num(rct),"build_number":num(bn),"note":note}))' "$@" >> "$LOG"; }

native_sample(){ local mode="$1" rep="$2" slot start finish cap
  cap="$RUN_ID-$mode-$rep"; slot="$(wipe "$WS/$mode")"
  start="$(ts_ms)"
  case "$mode" in
    native)        (cd "$SRC/rust" && CARGO_TARGET_DIR="$slot" cargo build --workspace --locked >/dev/null) ;;
    native-debug0) (cd "$SRC/rust" && CARGO_TARGET_DIR="$slot" CARGO_PROFILE_DEV_DEBUG=0 cargo build --workspace --locked >/dev/null) ;;
    native-lld)    (cd "$SRC/rust" && CARGO_TARGET_DIR="$slot" RUSTFLAGS="$LLD_FLAGS" cargo build --workspace --locked >/dev/null) ;;
  esac
  finish="$(ts_ms)"
  emit "$mode" "$rep" "$((finish-start))" "$cap" - - - - - - "plain cargo, no ib_console"; }

ib_warm_sample(){ local rep="$1"
  # two `local`s: within one `local`, an earlier assignment on the same line has
  # not taken effect yet, so `cap` would be built from an empty `rep`.
  local cap="$RUN_ID-ib-warm-identical-$rep" target log start finish c
  target="$(wipe "$WS/ib")"; log="$RAW/$cap.console.txt"
  start="$(ts_ms)"; ib_build "$cap" "$target" "$log"; finish="$(ts_ms)"
  read -r -a c <<<"$(ib_counters "$cap" "$log")"
  emit ib-warm-identical "$rep" "$((finish-start))" "$cap" "${c[0]}" "${c[1]}" "${c[2]}" "${c[3]}" "${c[4]}" "${c[5]}" \
    "cache-only profile; identical source; same target path; contents wiped before build"
  [[ "${c[3]}" == 0 ]] || { echo "$cap: remote_tasks=${c[3]} under a local_only profile" >&2; return 1; }; }

{ echo "run_id=$RUN_ID"; echo "base_revision=$BASE"; echo "samples_per_mode=$SAMPLES"
  echo "profile=$IB_PROFILE sha256=$PROFILE_SHA rustc_type=local_only ib_cache=true"
  echo "ib_console_args=${IB_CONSOLE_ARGS[*]} (no -f / --force-remote)"
  echo "ib_target_path=$WS/ib (stable; contents wiped before every sample)"
  echo "lld_flags=$LLD_FLAGS"
  echo "timed_command=cargo build --workspace --locked"
  echo "downloads_outside_timing=true"
  echo "gated_by_build_proof=false (the receipt schema has three modes; these are side measurements)"
} > "$OUT/method.txt"

# ---- warm the cache once, from an EMPTY cache, at the target path the warm
#      samples will use. The seed build is itself a cache-cold cache-only build.
"$CACHE_CLEAR" "$RAW/$RUN_ID-seed.cache-clear.txt" "$IB_CACHE_TOOL" user clear
SEED_CAP="$RUN_ID-seed-cold"; SEED_T="$(wipe "$WS/ib")"; SEED_LOG="$RAW/$SEED_CAP.console.txt"
s="$(ts_ms)"; ib_build "$SEED_CAP" "$SEED_T" "$SEED_LOG"; f="$(ts_ms)"
read -r -a sc <<<"$(ib_counters "$SEED_CAP" "$SEED_LOG")"
emit ib-cold-seed 0 "$((f-s))" "$SEED_CAP" "${sc[0]}" "${sc[1]}" "${sc[2]}" "${sc[3]}" "${sc[4]}" "${sc[5]}" \
  "cache emptied immediately before; populates the cache the warm samples read"

MODES=(native native-debug0 native-lld ib-warm-identical)
for rep in $(seq 1 "$SAMPLES"); do
  echo "== repetition $rep / $SAMPLES =="
  order=(); n=${#MODES[@]}
  for i in $(seq 0 $((n-1))); do order+=("${MODES[$(( (i + rep - 1) % n ))]}"); done
  printf '%s %s\n' "$rep" "${order[*]}" >> "$OUT/run-order.txt"
  for m in "${order[@]}"; do
    echo "-- $m rep $rep"
    if [[ "$m" == ib-warm-identical ]]; then ib_warm_sample "$rep"; else native_sample "$m" "$rep"; fi
  done
done

echo; echo "=== PHASE 2 RESULTS (counters from Incredibuild telemetry only) ==="
python3 - "$LOG" <<'PY'
import json,sys,statistics as st
rows=[json.loads(l) for l in open(sys.argv[1])]
print(f"{'mode':<20}{'n':>3}{'median_ms':>11}{'min':>9}{'max':>9}{'hits':>16}{'remote':>8}")
for m in dict.fromkeys(r["mode"] for r in rows):
    g=[r for r in rows if r["mode"]==m]; w=sorted(r["wall_ms"] for r in g)
    hits=sorted({str(r["cache_hits"])+"/"+str(r["cache_total"]) for r in g if r["cache_total"] is not None}) or ["-"]
    rem=sorted({str(r["remote_tasks"]) for r in g if r["remote_tasks"] is not None}) or ["-"]
    print(f"{m:<20}{len(g):>3}{int(st.median(w)):>11}{w[0]:>9}{w[-1]:>9}{','.join(hits):>16}{','.join(rem):>8}")
PY
echo "evidence: $OUT"
