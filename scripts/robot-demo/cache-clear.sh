#!/usr/bin/env bash
# Clear a build cache and transcribe the operation.
#
# Usage: cache-clear.sh <transcript-path> <cache-tool> <argv...>
#
# This transcript is the only thing `swf-cli robot-demo build-proof` accepts as
# evidence that a cache was emptied before a measured build. Nothing downstream
# asserts a cache state: the scope is derived from the argv recorded here, and a
# non-zero exit is recorded as-is, which makes the receipt fail closed rather
# than pass with an unverifiable claim.
#
# Deliberately NOT `set -e`: the tool's failure must be captured and written
# down, not swallowed. The script still exits with the tool's status, so the
# caller's own `set -e` stops the benchmark.
set -uo pipefail

[[ $# -ge 2 ]] || { echo "usage: cache-clear.sh <transcript> <tool> <argv...>" >&2; exit 2; }
OUT="$1"; shift
TOOL="$1"; shift

# GNU date's %3N is absent on BSD/macOS, where it would silently produce a
# timestamp ending in "3N" and the Rust parser would reject the transcript.
now_ms() {
  local now
  now="$(date +%s%3N)"
  if [[ "$now" =~ ^[0-9]+$ ]]; then
    printf '%s\n' "$now"
  else
    python3 -c 'import time; print(int(time.time() * 1000))'
  fi
}

mkdir -p "$(dirname "$OUT")"
started="$(now_ms)"
body="$("$TOOL" "$@" 2>&1)"
status=$?
completed="$(now_ms)"

# The tool name was shifted off before this point. Recording only "$*" produced
# transcripts like `argv=-rf /path` with no indication of WHICH tool ran, which
# made the transcript unable to evidence what it claims to evidence. Record the
# tool and the full command line, and keep argv for backward compatibility.
{
  echo "# swf-cache-clear v2"
  echo "tool=$TOOL"
  echo "command=$TOOL $*"
  echo "argv=$*"
  echo "exit_code=$status"
  echo "started_at_ms=$started"
  echo "completed_at_ms=$completed"
  echo "--- transcript ---"
  printf '%s\n' "$body"
} > "$OUT"

exit "$status"
