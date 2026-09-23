#!/usr/bin/env bash
# Preflight: verify toolchain, simulator backend, and IB/sandbox availability.
set -euo pipefail
cd "$(dirname "$0")/../.."
# shellcheck disable=SC1091
[ -f .env.local ] && { set -a; . ./.env.local; set +a; }

echo "== preflight =="

if command -v cargo >/dev/null; then
  echo "ok   cargo $(cargo --version | awk '{print $2}')"
else
  echo "FAIL cargo missing"; exit 1
fi

PYTHON="${ROBOT_DEMO_PYTHON:-python3}"
if command -v "$PYTHON" >/dev/null; then
  echo "ok   python: $("$PYTHON" --version 2>&1)"
else
  echo "FAIL python missing"; exit 1
fi

if [ -x demo/robot-sim/.venv/bin/python3 ]; then
  echo "ok   sim venv present (demo/robot-sim/.venv)"
  if demo/robot-sim/.venv/bin/python3 -c "import robosuite, mujoco" 2>/dev/null; then
    echo "ok   robosuite $(demo/robot-sim/.venv/bin/python3 -c 'import robosuite;print(robosuite.__version__)') / mujoco $(demo/robot-sim/.venv/bin/python3 -c 'import mujoco;print(mujoco.__version__)')"
  else
    echo "warn robosuite import failed in venv — robosuite backend unavailable, mock backend only"
  fi
else
  echo "warn no sim venv — robosuite backend unavailable, mock backend only"
fi

if [ ! -f rust/ib_profile.xml ]; then
  echo "FAIL rust/ib_profile.xml missing — refusing an unprofiled Incredibuild run"
  exit 1
fi
"$PYTHON" -c 'import xml.etree.ElementTree as ET; ET.parse("rust/ib_profile.xml")'
echo "ok   rust/ib_profile.xml is well-formed (project-level profile)"

if command -v ib_console >/dev/null 2>&1; then
  echo "ok   ib_console detected — runner invokes it; cache configuration/reuse must be verified separately"
else
  if [ "${REQUIRE_IB:-0}" = "1" ]; then
    echo "FAIL REQUIRE_IB=1 but ib_console is unavailable"
    exit 1
  fi
  echo "warn ib_console not found — native cargo baseline (labeled; no acceleration claimed)"
fi

if [ -n "${ISLO_SANDBOX_KEY:-}" ]; then
  echo "info islo credentials present; these scripts still use local worktrees (no remote dispatcher implemented)"
else
  echo "info no islo sandbox key — runners use local isolated git worktrees"
fi

echo "== preflight complete =="
