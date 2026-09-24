#!/usr/bin/env bash
# Controlled measurement of parent-warmed Incredibuild Build Cache reuse for a
# Rust workspace, with or without distribution.
#
# Run this on the EC2 Incredibuild Initiator, from a clean repository clone.
# It deliberately empties the Incredibuild Build Cache before each cold sample
# and each parent seed. Read CACHE CLEAR BLAST RADIUS below before assuming
# what "empties" means here: it is not scoped to the invoking user.
#
# ACCELERATION MODE -- the first thing to decide, and the default has changed.
#
# Incredibuild offers two independent accelerations, and the profile schema at
# /opt/incredibuild/data/ib_profile.xsd makes that independence explicit: a
# <process> carries a `type` (whose enumeration includes both allow_remote and
# local_only) AND a separate <ib_cache enabled="..."/> child. So rustc can be
# cached without ever being distributed.
#
#   IB_ACCEL=cache-only  (DEFAULT) rust/ib_profile.cache-only.xml declares
#                        rustc type="local_only" with ib_cache enabled. Nothing
#                        is sent to a helper. Build History then reports
#                        remote_tasks=0 and remote_core_time=0, and build-proof
#                        is run with --distribution excluded, which REFUSES the
#                        receipt if either is anything else. Any speed-up the
#                        run measures is the Build Cache's, and nothing else's.
#
#   IB_ACCEL=distributed rust/ib_profile.xml declares rustc type="allow_remote".
#                        Cache and helpers both act, the wall time mixes them,
#                        and build-proof runs with --distribution required.
#
# cache-only is the default because a mixed measurement cannot answer "how much
# did the cache do", which is the question this benchmark exists to answer. The
# distributed variant is still a single environment variable away, and both are
# recorded verbatim in method.txt.
#
# WORKSPACE DISCIPLINE -- read this before changing any target directory.
#
# Incredibuild's Build Cache key includes the rustc command line, and a rustc
# command line contains `--out-dir`, i.e. the CARGO_TARGET_DIR path. The per-task
# Build Cache report (`--build-cache-report-all-miss`, written to ib_hm.log)
# states this directly: two otherwise identical compilations that differ only in
# their target directory are recorded as
#     MISS: nothing_relevant_in_local_cache
#     REASON: command line mismatch
#
# This script used to call a `fresh_target()` that mktemp'd a NEW target
# directory for every sample. Every compilation therefore carried a never-seen
# `--out-dir`, the key never matched, and reuse was ~0% BY CONSTRUCTION -- in
# every mode, no matter how warm the cache was. The `ib-parent-warm` mode could
# not demonstrate reuse even in principle: it was measuring a cache it had
# disabled. Measured consequence: 15/15 samples reported total=52, hits=1.
#
# The replacement is `disposable_workspace`: the target PATH is held stable and
# its CONTENTS are wiped before every build. cargo then finds nothing, has no
# incremental state, and must re-issue every rustc invocation -- so anything
# that completes instantly came from the Incredibuild Build Cache and from
# nothing else. That is precisely the "disposable workspace, reusable
# compilation" configuration, and in the isolation experiment it moved the
# counters from 52 total / 1 hit to 52 total / 52 hits.
#
# ib-cold and ib-parent-warm deliberately share ONE stable path (WS_IB). They
# must differ only in the state of the cache, never in the cache key. A cold
# sample is made cold by emptying the cache, never by moving the target
# directory -- moving it would make "cold" cold for two different reasons and
# would make the cold/warm delta uninterpretable.
#
# CACHE CLEAR BLAST RADIUS -- this script used to describe it wrongly.
#
# The operator gate below used to read "this benchmark clears only qa_user's
# local IB cache". That is FALSE on this Incredibuild build and the correction
# is measured, not argued: /opt/incredibuild/management/build_avoid_cache.sh
# runs `rm -rf /etc/incredibuild/cache/build_cache/shared/*` UNCONDITIONALLY,
# whatever scope argument it is given, and an isolation experiment watched a
# `user clear` take that directory from 102880 KB / 105 entries to 8 KB / 0.
# The rustc ib_cache store IS that shared directory; the per-user directory
# /etc/incredibuild/cache/build_avoid/<user>.<uid> that `user` names is the
# CCACHE store for C/C++ and never holds a Rust entry.
#
# So every clear below is MACHINE-WIDE and destroys every user's Rust Build
# Cache on this initiator. Nothing downstream is changed to hide that: the
# transcript still records the argv `user clear`, and swf-cli still derives the
# scope from that argv and prints it as local-user, because the scope it can
# derive is the scope that was REQUESTED. The gate text and method.txt now
# state the measured blast radius so an operator consents to what actually
# happens. Do not run this on a machine someone else is building on.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"
# .env.local fills unset values only; the operator's own environment wins.
# shellcheck source=scripts/robot-demo/env-local.sh
. "$ROOT/scripts/robot-demo/env-local.sh"
RUN_ID="${1:-ib-proof-$(date +%s)}"
SAMPLES="${IB_SAMPLES:-5}"
PATCH="${ROBOT_DEMO_PATCH_FILE:-$ROOT/demo/fallback-patch.diff}"
BASE="${ROBOT_DEMO_BASE_REVISION:-$(git rev-parse HEAD)}"
# Every artifact of this benchmark lives under the run id that was passed in,
# which is not necessarily the caller's own run id (the e2e script appends
# "-build"). Nothing may re-derive these paths by string surgery: the script
# prints them and records them in method.txt, and that is the only location.
OUT="$ROOT/evidence/$RUN_ID/build-proof"
RAW="$OUT/raw"
SAMPLE_LOG="$OUT/samples.jsonl"
PROOF="$OUT/receipt.json"
IB_CACHE_TOOL="/opt/incredibuild/management/build_avoid_cache.sh"
IB_STATS_TOOL="/opt/incredibuild/management/show_build_cache_statistics.sh"
# Every cache clear goes through this wrapper, which writes the transcript the
# Rust verifier reads. Clearing the cache without it produces a sample that
# build-proof rejects, which is the point: an unrecorded clear is unknown.
CACHE_CLEAR="$ROOT/scripts/robot-demo/cache-clear.sh"
# The proof CLI is built into a target directory this script names itself. The
# runners export CARGO_TARGET_DIR at throwaway directories, so an inherited one
# would otherwise send the bootstrap build somewhere else and leave PROOF_CLI
# pointing at whatever stale binary happens to sit in rust/target/debug.
PROOF_CLI_TARGET="$ROOT/rust/target"
PROOF_CLI="$PROOF_CLI_TARGET/debug/swf-cli"

# ib_console's `-f` is `--force-remote`: "force allow_remote tasks to remote
# helpers". It does not add remote capacity, it REPLACES local capacity -- the
# initiator's own cores sit idle while every remotable task crosses the network.
# The previous run passed it unconditionally, which is why Build History
# recorded maxInitiatorCores=0 and local_tasks=10, and why "Incredibuild is 2x
# slower" was the wall time of a deliberately handicapped configuration rather
# than a measurement of Incredibuild. The default is now OFF. Setting
# IB_FORCE_REMOTE=1 re-enables it as an explicitly labelled variant, and the
# exact argv used is recorded per sample and in method.txt either way.
IB_FORCE_REMOTE="${IB_FORCE_REMOTE:-0}"
[[ "$IB_FORCE_REMOTE" =~ ^[01]$ ]] || {
  echo "IB_FORCE_REMOTE must be 0 or 1" >&2; exit 2;
}

# See ACCELERATION MODE at the top. The profile this selects decides whether
# rustc may leave the machine at all, and it decides which contract the final
# verifier is run under -- the two are set together, here, so a cache-only run
# cannot be checked by the distribution contract or the reverse.
IB_ACCEL="${IB_ACCEL:-cache-only}"
IB_PROFILE_DISTRIBUTED="$ROOT/rust/ib_profile.xml"
IB_PROFILE_CACHE_ONLY="$ROOT/rust/ib_profile.cache-only.xml"
case "$IB_ACCEL" in
  cache-only)
    IB_PROFILE="$IB_PROFILE_CACHE_ONLY"
    IB_EXPECTED_RUSTC_TYPE=local_only
    IB_DISTRIBUTION_CONTRACT=excluded
    ;;
  distributed)
    IB_PROFILE="$IB_PROFILE_DISTRIBUTED"
    IB_EXPECTED_RUSTC_TYPE=allow_remote
    IB_DISTRIBUTION_CONTRACT=required
    ;;
  *)
    echo "IB_ACCEL must be cache-only or distributed (got '$IB_ACCEL')" >&2; exit 2;
    ;;
esac
# -f forces allow_remote tasks onto helpers. Under a local_only profile there
# are no allow_remote tasks to force, so the combination is not a variant, it
# is a contradiction, and silently ignoring it would leave method.txt claiming
# a configuration that was never run.
if [[ "$IB_ACCEL" == cache-only && "$IB_FORCE_REMOTE" == 1 ]]; then
  echo "IB_FORCE_REMOTE=1 contradicts IB_ACCEL=cache-only: the cache-only profile" >&2
  echo "declares rustc local_only, so there is nothing for --force-remote to force." >&2
  exit 2
fi

# A single-sample rehearsal of the mechanism, for checking that the modes
# really do produce different cache counters before handing the machine to a
# measured run. It is NOT a measurement and cannot be mistaken for one: it
# refuses to call build-receipt or build-proof at all, so a smoke run produces
# no receipt for anything downstream to read, and it drops a marker file
# saying so next to the samples it did write.
IB_SMOKE="${IB_SMOKE:-0}"
[[ "$IB_SMOKE" =~ ^[01]$ ]] || { echo "IB_SMOKE must be 0 or 1" >&2; exit 2; }
if [[ "$IB_SMOKE" == 1 ]]; then
  MIN_SAMPLES=1
else
  MIN_SAMPLES=5
fi

[[ "$RUN_ID" =~ ^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$ ]] || {
  echo "invalid run id" >&2; exit 2;
}
[[ "$SAMPLES" =~ ^[0-9]+$ && "$SAMPLES" -ge "$MIN_SAMPLES" ]] || {
  echo "IB_SAMPLES must be at least $MIN_SAMPLES" >&2
  [[ "$IB_SMOKE" == 1 ]] || echo "  (IB_SMOKE=1 lowers this to 1, and then refuses to produce a receipt)" >&2
  exit 2
}
# The variable name is kept because operators and runbooks already export it,
# but the sentence it gates has been corrected. See CACHE CLEAR BLAST RADIUS.
[[ "${IB_ALLOW_CLEAR_USER_CACHE:-0}" == 1 ]] || {
  echo "set IB_ALLOW_CLEAR_USER_CACHE=1 to consent to the cache clears this benchmark performs." >&2
  echo "MEASURED BLAST RADIUS: build_avoid_cache.sh rm -rf's" >&2
  echo "/etc/incredibuild/cache/build_cache/shared/* unconditionally, whatever scope it is" >&2
  echo "given. That directory is the rustc Build Cache for EVERY user of this initiator," >&2
  echo "so each clear below is machine-wide, not scoped to $(id -un)." >&2
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
[[ -x "$CACHE_CLEAR" ]] || {
  echo "$CACHE_CLEAR is missing or not executable; cache clears would be unrecorded" >&2
  exit 2
}
[[ ! -e "$OUT" ]] || { echo "refusing to reuse $OUT" >&2; exit 2; }
mkdir -p "$RAW"
: > "$SAMPLE_LOG"
echo "build-proof evidence directory: $OUT"
echo "build-proof receipt will be written to: $PROOF"
echo "acceleration mode: $IB_ACCEL (profile $IB_PROFILE, distribution contract: $IB_DISTRIBUTION_CONTRACT)"

# Check both profiles against the schema Incredibuild ships, and against each
# other, BEFORE anything is cleared or built.
#
# xmllint is not installed on this grid, so this is not a full XSD validation
# and does not claim to be. What it does check is everything the two files can
# get wrong here, and it reads the rules out of the XSD at run time rather than
# repeating them from memory:
#   * every <process type="..."> is in type_type's enumeration AS THE XSD ON
#     THIS MACHINE DECLARES IT -- so "local_only is a legal value" is read from
#     the installed schema, not asserted by this script;
#   * every <process> carries the <ib_cache enabled="..."/> the XSD requires,
#     with a boolean value;
#   * the selected profile declares rustc with exactly the type this mode
#     depends on;
#   * and the two profiles are structurally IDENTICAL apart from that one
#     attribute -- same globals, same process list, same ib_cache settings --
#     so the cache-only run and the distributed run differ by one word and
#     cannot quietly drift into differing by more.
# The transcript is retained; a failure aborts before the first build.
IB_PROFILE_XSD="${IB_PROFILE_XSD:-/opt/incredibuild/data/ib_profile.xsd}"
[[ -r "$IB_PROFILE_XSD" ]] || {
  echo "cannot read the Incredibuild profile schema at $IB_PROFILE_XSD" >&2
  echo "refusing to measure with a profile nothing has been checked against" >&2
  exit 2
}
python3 - "$IB_PROFILE_XSD" "$IB_PROFILE_DISTRIBUTED" "$IB_PROFILE_CACHE_ONLY" \
         "$IB_PROFILE" "$IB_EXPECTED_RUSTC_TYPE" <<'PYCHECK' | tee "$OUT/profile-check.txt"
import sys, xml.etree.ElementTree as ET

xsd_path, distributed, cache_only, selected, expected_rustc_type = sys.argv[1:6]
XS = "{http://www.w3.org/2001/XMLSchema}"

xsd = ET.parse(xsd_path).getroot()
allowed = None
for simple in xsd.iter(f"{XS}simpleType"):
    if simple.get("name") == "type_type":
        allowed = [e.get("value") for e in simple.iter(f"{XS}enumeration")]
if not allowed:
    raise SystemExit(f"FAIL {xsd_path} declares no type_type enumeration")
print(f"xsd={xsd_path}")
print(f"xsd_type_type_enumeration={','.join(allowed)}")

def shape(path):
    root = ET.parse(path).getroot()
    globals_el = root.find("globals")
    procs = {}
    for proc in root.iter("process"):
        name = proc.get("filename")
        cache = proc.find("ib_cache")
        if cache is None or cache.get("enabled") is None:
            raise SystemExit(f"FAIL {path}: <process filename={name!r}> has no "
                             "<ib_cache enabled=...>, which the XSD makes required")
        if cache.get("enabled") not in ("true", "false", "1", "0"):
            raise SystemExit(f"FAIL {path}: ib_cache enabled={cache.get('enabled')!r} "
                             "is not a boolean")
        if proc.get("type") not in allowed:
            raise SystemExit(f"FAIL {path}: <process filename={name!r} "
                             f"type={proc.get('type')!r}> is not in the XSD enumeration "
                             f"{allowed}")
        procs[name] = (proc.get("type"), cache.get("enabled"))
    return (dict(sorted((globals_el.attrib if globals_el is not None else {}).items())),
            root.get("version"), procs)

dg, dv, dp = shape(distributed)
cg, cv, cp = shape(cache_only)
if (dg, dv) != (cg, cv):
    raise SystemExit("FAIL the two profiles differ outside the rustc type attribute "
                     f"(globals/version): {dg},{dv} vs {cg},{cv}")
if sorted(dp) != sorted(cp):
    raise SystemExit(f"FAIL the two profiles declare different processes: "
                     f"{sorted(dp)} vs {sorted(cp)}")
for name in dp:
    if dp[name][1] != cp[name][1]:
        raise SystemExit(f"FAIL {name}: ib_cache enabled differs between the profiles "
                         f"({dp[name][1]} vs {cp[name][1]})")
    if name != "rustc" and dp[name][0] != cp[name][0]:
        raise SystemExit(f"FAIL {name}: type differs between the profiles and only "
                         f"rustc's may ({dp[name][0]} vs {cp[name][0]})")
if dp.get("rustc", (None,))[0] != "allow_remote":
    raise SystemExit(f"FAIL {distributed}: rustc type is {dp.get('rustc')}, expected allow_remote")
if cp.get("rustc", (None,))[0] != "local_only":
    raise SystemExit(f"FAIL {cache_only}: rustc type is {cp.get('rustc')}, expected local_only")

sg, sv, sp = shape(selected)
if sp.get("rustc", (None,))[0] != expected_rustc_type:
    raise SystemExit(f"FAIL selected profile {selected}: rustc type is "
                     f"{sp.get('rustc')}, this mode requires {expected_rustc_type}")
if sp["rustc"][1] not in ("true", "1"):
    raise SystemExit(f"FAIL selected profile {selected}: rustc ib_cache is not enabled, "
                     "so there is no cache to measure")
print(f"distributed_profile={distributed} rustc={dp['rustc'][0]} ib_cache={dp['rustc'][1]}")
print(f"cache_only_profile={cache_only} rustc={cp['rustc'][0]} ib_cache={cp['rustc'][1]}")
print(f"selected_profile={selected} rustc={sp['rustc'][0]} ib_cache={sp['rustc'][1]}")
print("structurally_identical_apart_from_rustc_type=true")
print("OK")
PYCHECK
IB_PROFILE_SHA="$(sha256sum "$IB_PROFILE" | awk '{print $1}')"
echo "selected_profile_sha256=$IB_PROFILE_SHA" | tee -a "$OUT/profile-check.txt"

# Build the Rust evidence tool before the measured experiment. This is a plain
# cargo build, not an ib_console build, so it never consults or populates the
# Incredibuild Build Cache; and it targets $PROOF_CLI_TARGET, which is not any
# of the workspace slots below, so its --out-dir cannot collide with a measured
# build's cache key either.
CARGO_TARGET_DIR="$PROOF_CLI_TARGET" \
  cargo build --manifest-path "$ROOT/rust/Cargo.toml" --locked -q -p swf-cli
[[ -x "$PROOF_CLI" ]] || {
  echo "cargo did not produce $PROOF_CLI" >&2; exit 2;
}
# Fail before clearing any cache if the freshly built CLI cannot gate the proof.
"$PROOF_CLI" robot-demo build-proof --help >/dev/null 2>&1 || {
  echo "$PROOF_CLI has no 'robot-demo build-proof' subcommand." >&2
  echo "The verifier is the final gate; refusing to run an ungated experiment." >&2
  exit 2
}

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

# Incredibuild loads $PWD/ib_profile.xml with the highest precedence and every
# build below runs from <worktree>/rust, so the profile that is actually in
# force is the one sitting THERE. Install the selected one over each worktree's
# own copy, rather than passing -p and hoping precedence lands the right way
# round, and record the digest of the bytes that were installed. The checked-out
# repository is never edited: the worktrees are disposable and removed on exit.
install_profile() {
  local slot="$1"
  # Two `local`s on purpose: within one `local`, an earlier assignment on the
  # same line has not taken effect yet, so `destination` would be built from an
  # empty `slot` and the profile would be written to /rust/ib_profile.xml.
  local destination="$slot/rust/ib_profile.xml"
  cp "$IB_PROFILE" "$destination"
  local installed
  installed="$(sha256sum "$destination" | awk '{print $1}')"
  [[ "$installed" == "$IB_PROFILE_SHA" ]] || {
    echo "profile install to $destination changed the bytes: $installed" >&2
    return 1
  }
  printf 'installed=%s sha256=%s from=%s\n' "$destination" "$installed" "$IB_PROFILE" \
    >> "$OUT/profile-check.txt"
}
install_profile "$PARENT_SRC"
install_profile "$CANDIDATE_SRC"

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
  local history="$1" caption="$2" output="$3" clear="$4"
  local build_number
  build_number="$("$PROOF_CLI" robot-demo build-history \
    --input "$history" --caption "$caption" --build-number-only)"
  "$IB_STATS_TOOL" "$build_number" > "$output"
  # Fail now if this version's output cannot be parsed; never record inferred
  # data. The real clear transcript is passed because build-sample refuses to
  # emit an Incredibuild sample without one.
  "$PROOF_CLI" robot-demo build-sample \
    --mode ib-cold --repetition 1 --wall-ms 1 --started-at-ms 1 --caption "$caption" \
    --source-revision parser-check --history "$history" --cache "$output" \
    --cache-clear "$clear" >/dev/null
}

# Stable target paths, one per workspace slot. The PATH is part of the Build
# Cache key (see WORKSPACE DISCIPLINE at the top), so it is fixed for the whole
# run and every sample that is meant to be able to share cache entries uses the
# same one. WORK is created once per run, so these are stable across samples and
# unique across runs.
WS_ROOT="$WORK/ws"
WS_NATIVE="$WS_ROOT/native"
# ib-cold, parent-seed and ib-parent-warm all build here, on purpose.
WS_IB="$WS_ROOT/ib"
mkdir -p "$WS_NATIVE" "$WS_IB"

# Wipe a workspace's CONTENTS while keeping its PATH: the disposable workspace.
# cargo is left with no target directory and no incremental state, so it must
# re-issue every rustc invocation in the graph; the Incredibuild Build Cache is
# then the only thing that can make any of them fast. Callers must invoke this
# BEFORE starting the clock -- the wipe is setup, not part of the measurement --
# and must not wipe between a build and the reading of its counters.
disposable_workspace() {
  local slot="$1"
  rm -rf "$slot"
  mkdir -p "$slot"
  # Echoed rather than assumed, so a caller cannot silently build somewhere else.
  printf '%s\n' "$slot"
}

native_sample() {
  local rep="$1" target start finish caption
  caption="$RUN_ID-native-$rep"
  # Its own slot: native cargo never touches the Build Cache, so sharing WS_IB
  # would only risk a native build leaving state a measured IB build inherits.
  target="$(disposable_workspace "$WS_NATIVE")"
  start="$(ts_ms)"
  (cd "$CANDIDATE_SRC/rust" && CARGO_TARGET_DIR="$target" cargo build --workspace --locked)
  finish="$(ts_ms)"
  "$PROOF_CLI" robot-demo build-sample \
    --mode native --repetition "$rep" --wall-ms "$((finish-start))" \
    --started-at-ms "$start" \
    --caption "$caption" --source-revision "$CANDIDATE_ID" >> "$SAMPLE_LOG"
}

# Clear the invoking user's own cache, recording the operation. The transcript
# path is derived from the caption of the build the clear prepares, so the
# sample that follows can cite exactly this operation.
clear_cache() {
  local caption="$1"
  "$CACHE_CLEAR" "$RAW/$caption.cache-clear.txt" "$IB_CACHE_TOOL" user clear
}

# The one place ib_console's argument list is decided, so that every build in
# the run is launched identically and the argv can be written down verbatim
# rather than described. --build-cache-report-all-miss is not optional here:
# capture_cache_report reads the report it produces and refuses a sample whose
# per-task records disagree with the aggregate counters.
IB_CONSOLE_ARGS=(--build-cache-local-user --build-cache-report-all-miss)
if [[ "$IB_FORCE_REMOTE" == 1 ]]; then
  # Labelled variant, never the default. See the IB_FORCE_REMOTE comment above.
  IB_CONSOLE_ARGS=(-f "${IB_CONSOLE_ARGS[@]}")
fi

ib_build() {
  local source="$1" target="$2" caption="$3" log="$4"
  # Written before the build, so a build that dies still leaves behind the exact
  # command line it died running.
  printf '%s\n' "ib_console -c $caption ${IB_CONSOLE_ARGS[*]} cargo build --workspace --locked" \
    > "$RAW/$caption.ib-argv.txt"
  (
    cd "$source/rust"
    CARGO_TARGET_DIR="$target" ib_console \
      -c "$caption" \
      "${IB_CONSOLE_ARGS[@]}" \
      cargo build --workspace --locked
  ) 2>&1 | tee "$log"
}

# Retain the per-task Build Cache report and cross-check it against the
# aggregate the statistics tool read out of the build report DB.
#
# The report is the ground truth the aggregate is summarising: one block per
# rustc invocation, each ending in HIT: or MISS: with a REASON. Two independent
# views of the same build that must agree; if they do not, the counters this
# receipt would record are not describing this build, and the sample is refused
# rather than written down.
#
# It is also the only artifact that explains the empty-cache floor. On a
# genuinely emptied cache this build still reports exactly one hit, and the
# report names it: cargo invokes `rustc -vV` twice, the first MISSes and stores,
# the second HITs the entry the first just wrote. It is an intra-build self-hit
# and carries no information about prior cache state.
capture_cache_report() {
  local caption="$1" log="$2" cache="$3"
  local source_path report derived_hits derived_misses aggregate
  source_path="$(sed -n "s/.*Build Cache report is '\([^']*\)'.*/\1/p" "$log" | tail -n 1)"
  [[ -n "$source_path" && -r "$source_path" ]] || {
    echo "$caption: ib_console printed no readable Build Cache report path" >&2
    return 1
  }
  report="$RAW/$caption.cache-report.txt"
  cp "$source_path" "$report"
  derived_hits="$(grep -c '^HIT:' "$report" || true)"
  derived_misses="$(grep -c '^MISS:' "$report" || true)"
  aggregate="$(sed -n 's/^hits=//p' "$cache")"
  [[ -n "$aggregate" ]] || {
    echo "$caption: $IB_STATS_TOOL output has no hits= line" >&2
    return 1
  }
  [[ "$derived_hits" == "$aggregate" ]] || {
    echo "$caption: Build Cache report shows $derived_hits hit(s) but the build report DB" >&2
    echo "  reports $aggregate; refusing to record counters two sources disagree about" >&2
    return 1
  }
  printf '%s hits=%s misses=%s source=%s\n' \
    "$caption" "$derived_hits" "$derived_misses" "$source_path" \
    >> "$OUT/cache-report-per-task.txt"
}

ib_sample() {
  local mode="$1" rep="$2" source="$3"
  local target caption log history cache clear start finish
  caption="$RUN_ID-$mode-$rep"
  # The stable shared slot, contents wiped: cargo must recompile everything, and
  # the cache key is identical to the one the parent seed populated. ib-cold uses
  # the same path on purpose -- the clear, not the path, is what makes it cold.
  target="$(disposable_workspace "$WS_IB")"
  log="$RAW/$caption.console.txt"
  history="$RAW/$caption.history.json"
  cache="$RAW/$caption.cache.txt"
  clear="$RAW/$caption.cache-clear.txt"
  [[ -f "$clear" ]] || {
    echo "no cache-clear transcript for $caption; refusing to record its cache state" >&2
    return 1
  }
  local seed_args=()
  if [[ "$mode" == "ib-parent-warm" ]]; then
    # Written by seed_parent_cache immediately before this call.
    # If the seed built anywhere other than this slot, its cache entries carry a
    # different --out-dir, cannot key-match this build, and "the parent warmed
    # the cache" would be false however good the numbers looked.
    [[ "${SEED_TARGET:-}" == "$target" ]] || {
      echo "$caption: parent seed built in '${SEED_TARGET:-<unset>}' but this build uses" >&2
      echo "  '$target';" >&2
      echo "  the Build Cache key includes the target path, so no reuse is possible" >&2
      return 1
    }
    seed_args=(
      --parent-seed-history "$SEED_HISTORY"
      --parent-seed-cache "$SEED_CACHE"
      --parent-seed-caption "$SEED_CAPTION"
      --parent-seed-revision "$BASE"
      --parent-seed-started-at-ms "$SEED_STARTED_MS"
    )
  fi
  start="$(ts_ms)"
  ib_build "$source" "$target" "$caption" "$log"
  finish="$(ts_ms)"
  wait_for_history "$caption" "$history"
  cache_stats "$history" "$caption" "$cache" "$clear"
  capture_cache_report "$caption" "$log" "$cache"
  "$PROOF_CLI" robot-demo build-sample \
    --mode "$mode" --repetition "$rep" --wall-ms "$((finish-start))" \
    --started-at-ms "$start" \
    --caption "$caption" --source-revision "$CANDIDATE_ID" \
    --history "$history" --cache "$cache" \
    --cache-clear "$clear" ${seed_args[@]+"${seed_args[@]}"} >> "$SAMPLE_LOG"
  # Deliberately NOT removed: the wipe happens immediately before the next build,
  # so nothing in between can repopulate the slot, and the artifacts of the last
  # build remain available for inspection if the run is investigated.
}

# Publishes SEED_* for the ib-parent-warm sample that follows it. The warm
# sample cites this build by caption, start time and its own cache counters, so
# "the parent warmed the cache" is checked rather than assumed.
seed_parent_cache() {
  local rep="$1" target log clear
  # The clear that preceded this seed is the one taken for the warm sample.
  clear="$RAW/$RUN_ID-ib-parent-warm-$rep.cache-clear.txt"
  SEED_CAPTION="$RUN_ID-parent-seed-$rep"
  # The SAME slot the measured warm build will use, wiped. This is the whole
  # mechanism: identical target path => identical --out-dir => the cache entries
  # this seed stores are the ones the next build can key-match.
  SEED_TARGET="$(disposable_workspace "$WS_IB")"
  target="$SEED_TARGET"
  log="$RAW/$SEED_CAPTION.console.txt"
  SEED_HISTORY="$RAW/$SEED_CAPTION.history.json"
  SEED_CACHE="$RAW/$SEED_CAPTION.cache.txt"
  SEED_STARTED_MS="$(ts_ms)"
  ib_build "$PARENT_SRC" "$target" "$SEED_CAPTION" "$log"
  wait_for_history "$SEED_CAPTION" "$SEED_HISTORY"
  cache_stats "$SEED_HISTORY" "$SEED_CAPTION" "$SEED_CACHE" "$clear"
  capture_cache_report "$SEED_CAPTION" "$log" "$SEED_CACHE"
  # Not removed: ib_sample wipes this same path immediately before the measured
  # build, which is what makes that build a disposable workspace.
}

{
  echo "run_id=$RUN_ID"
  echo "base_revision=$BASE"
  echo "candidate_identity=$CANDIDATE_ID"
  echo "samples_per_mode=$SAMPLES"
  echo "timed_command=cargo build --workspace --locked"
  echo "downloads_outside_timing=true"
  echo "acceleration_mode=$IB_ACCEL"
  echo "ib_profile_installed=$IB_PROFILE"
  echo "ib_profile_sha256=$IB_PROFILE_SHA"
  echo "ib_profile_rustc_type=$IB_EXPECTED_RUSTC_TYPE (checked against the type_type"
  echo "                      enumeration in $IB_PROFILE_XSD; see profile-check.txt)"
  echo "distribution_contract=$IB_DISTRIBUTION_CONTRACT (passed to build-proof as"
  echo "                      --distribution; 'excluded' REFUSES the receipt if any IB"
  echo "                      sample or parent seed reports a remote task or remote core"
  echo "                      time, so a cache-only claim fails on the first leaked task)"
  echo "ib_console_args=${IB_CONSOLE_ARGS[*]}"
  echo "force_remote=$IB_FORCE_REMOTE (1 means -f/--force-remote: remotable tasks"
  echo "                               REPLACE the initiator's local cores)"
  echo "workspace_discipline=stable target path, contents wiped before every build"
  echo "native_target_path=$WS_NATIVE"
  echo "ib_target_path=$WS_IB   (ib-cold, parent-seed and ib-parent-warm share it)"
  echo "cache_key_note=the Build Cache key includes rustc --out-dir, i.e. the target"
  echo "               path; cold and warm therefore differ only in cache state"
  echo "empty_cache_hit_floor=1 (cargo runs 'rustc -vV' twice; the second hits the"
  echo "                         entry the first stored -- an intra-build self-hit,"
  echo "                         see raw/*.cache-report.txt for the two blocks;"
  echo "                         passed to build-proof as --empty-cache-hit-floor 1,"
  echo "                         which also requires warm samples to exceed it)"
  echo "cache_scope_requested=local-user (recorded per clear in raw/*.cache-clear.txt;"
  echo "                                   the receipt derives it, this line is not evidence)"
  echo "cache_clear_measured_blast_radius=machine-wide: build_avoid_cache.sh rm -rf's"
  echo "                      /etc/incredibuild/cache/build_cache/shared/* whatever scope it"
  echo "                      is given, and that directory is the rustc Build Cache for every"
  echo "                      user of this initiator. The 'user' argv above is what was"
  echo "                      REQUESTED; this line is what was measured to happen."
  echo "build_cache_local_user_flag_note=--build-cache-local-user selects"
  echo "                      /etc/incredibuild/cache/build_avoid/<user>.<uid>, the CCACHE"
  echo "                      store for C/C++. It was measured to stay at 140 KB across 21"
  echo "                      Rust builds, i.e. it is inert for rustc. It is kept so the"
  echo "                      argv is unchanged from earlier runs, not because it isolates."
  echo "run_order=rotating native/ib-cold/ib-parent-warm"
  echo "per_task_cache_report=raw/<caption>.cache-report.txt (cross-checked against"
  echo "                      the build report DB; a disagreement aborts the run)"
  echo "evidence_dir=$OUT"
  echo "receipt_path=$PROOF"
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
      # Plain cargo, own slot, wiped. No Incredibuild involvement at all.
      native)
        native_sample "$rep"
        ;;
      # CACHE-COLD: the user's Build Cache is emptied, then the candidate is
      # built in the shared slot. Same target path as the warm samples, so the
      # only difference between this and ib-parent-warm is what is in the cache.
      # Expected counters: total=52, hits=1 -- the intra-build `rustc -vV`
      # self-hit documented above, which is the floor and not retained state.
      ib-cold)
        clear_cache "$RUN_ID-ib-cold-$rep"
        ib_sample ib-cold "$rep" "$CANDIDATE_SRC"
        ;;
      # CACHE-WARM, the disposable workspace: empty the cache, build the PARENT
      # revision in the shared slot to populate it, wipe the slot's contents
      # while keeping its path, then build the CANDIDATE there. cargo starts from
      # nothing both times, so every hit the candidate gets was served by the
      # Build Cache using entries the parent stored. Reuse is partial by
      # construction -- the candidate carries the patch, so the crates it touches
      # and everything downstream of them must genuinely recompile.
      ib-parent-warm)
        clear_cache "$RUN_ID-ib-parent-warm-$rep"
        seed_parent_cache "$rep"
        ib_sample ib-parent-warm "$rep" "$CANDIDATE_SRC"
        ;;
    esac
  done
done

# A smoke run stops HERE, before build-receipt, and says so in the evidence
# directory. It produced real builds and real counters -- that is the point of
# running it -- but too few samples to describe anything, so it must not leave
# behind a receipt that something downstream could pick up and quote. There is
# no flag that makes a smoke run produce one.
if [[ "$IB_SMOKE" == 1 ]]; then
  {
    echo "This directory is a SMOKE REHEARSAL of scripts/robot-demo/ib-benchmark.sh."
    echo "It ran $SAMPLES sample(s) per mode, which is below the $((5)) this benchmark"
    echo "requires of a measurement, and it deliberately did NOT run build-receipt or"
    echo "build-proof. There is no receipt.json and no summary.txt here, and nothing in"
    echo "this directory may be quoted as a measurement of anything."
    echo "acceleration_mode=$IB_ACCEL"
    echo "samples_per_mode=$SAMPLES"
  } > "$OUT/SMOKE-NOT-A-MEASUREMENT.txt"
  echo
  echo "=== SMOKE REHEARSAL: per-build Incredibuild counters ==="
  # Straight from the samples the runner recorded, which took them from the
  # Build History API and the build report DB. Nothing here is inferred from
  # wall time.
  python3 - "$SAMPLE_LOG" <<'PYSMOKE'
import json, sys
print(f"{'mode':<16}{'rep':>4}{'wall_ms':>9}{'remote':>8}{'hits':>6}{'miss':>6}  caption")
for line in open(sys.argv[1], encoding="utf-8"):
    s = json.loads(line)
    seed = s.get("parent_seed")
    if seed:
        print(f"{'  (parent seed)':<16}{'':>4}{'':>9}{seed['remote_tasks']:>8}"
              f"{seed['cache_hits']:>6}{seed['cache_misses']:>6}  {seed['build_caption']}")
    fmt = lambda v: "-" if v is None else v
    print(f"{s['mode']:<16}{s['repetition']:>4}{s['wall_ms']:>9}"
          f"{fmt(s.get('remote_tasks')):>8}{fmt(s.get('cache_hits')):>6}"
          f"{fmt(s.get('cache_misses')):>6}  {s['build_caption']}")
PYSMOKE
  echo
  echo "SMOKE REHEARSAL COMPLETE -- no receipt was written, by design."
  echo "  marker:   $OUT/SMOKE-NOT-A-MEASUREMENT.txt"
  echo "  samples:  $SAMPLE_LOG"
  echo "  per-task: $OUT/cache-report-per-task.txt"
  exit 0
fi

"$PROOF_CLI" robot-demo build-receipt \
  --samples "$SAMPLE_LOG" --run-id "$RUN_ID" \
  --candidate-revision "$CANDIDATE_ID" --parent-revision "$BASE" \
  --output "$PROOF"

[[ -f "$PROOF" ]] || {
  echo "build-receipt reported success but $PROOF does not exist" >&2; exit 1;
}

# The Rust verifier is the final gate. It computes only measured ratios and
# checks the receipt against the retained transcripts; it is a consistency
# check over self-reported evidence, not a proof that the evidence is real.
#
# --empty-cache-hit-floor 1 is the counterpart of the `empty_cache_hit_floor`
# line in method.txt above, and the two MUST agree: without it this script
# documents a hit floor of 1 and then calls a validator whose default demands
# 0, so every honest run would build for its full duration and die on its own
# last command. The floor is the verifier's to set, never the receipt's, and it
# is not a loosening: a warm sample must now report MORE than 1 hit, so raising
# the floor raises the bar the warm samples have to clear. The summary prints
# the floor it ran with and states that nothing corroborates that the hit it
# allowed was the intra-build `rustc -vV` self-hit -- the per-task Build Cache
# report in raw/*.cache-report.txt shows that it was, and is not yet retained
# with a digest the receipt is bound to.
# --distribution is the counterpart of `distribution_contract` in method.txt,
# and it is set from the same IB_ACCEL that chose the profile, so the contract
# the receipt is judged by can never disagree with the profile it was produced
# under. `excluded` is not the lenient setting: it demands remote_tasks == 0
# and remote_core_time == 0 from every IB sample AND every parent seed, so a
# cache-only run that leaked one task to a helper is refused here. Every cache
# check is identical under both settings.
"$PROOF_CLI" robot-demo build-proof --receipt "$PROOF" --min-samples "$SAMPLES" \
  --empty-cache-hit-floor 1 --distribution "$IB_DISTRIBUTION_CONTRACT" |
  tee "$OUT/summary.txt"

echo "verified build proof: $PROOF"
echo "                      (relative: evidence/$RUN_ID/build-proof/receipt.json)"
