#!/usr/bin/env bash
# Preflight: verify toolchain, simulator backend, and IB/sandbox availability.
set -euo pipefail
cd "$(dirname "$0")/../.."
# shellcheck disable=SC1091
[ -f .env.local ] && { set -a; . ./.env.local; set +a; }

echo "== preflight =="

command -v cargo >/dev/null && echo "ok   cargo $(cargo --version | awk '{print $2}')" || { echo "FAIL cargo missing"; exit 1; }

PYTHON="${ROBOT_DEMO_PYTHON:-python3}"
command -v "$PYTHON" >/dev/null && echo "ok   python: $("$PYTHON" --version 2>&1)" || { echo "FAIL python missing"; exit 1; }

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

if command -v ib_console >/dev/null 2>&1 || command -v ibwrap >/dev/null 2>&1; then
  echo "ok   Incredibuild wrapper detected — accelerated builds enabled"
else
  echo "warn Incredibuild wrapper not found — native cargo baseline (labeled; no acceleration claimed)"
fi

if [ -n "${ISLO_SANDBOX_KEY:-}" ]; then
  echo "ok   islo sandbox key present (remote runner dispatch enabled when configured)"
else
  echo "info no islo sandbox key — runners use local isolated git worktrees"
fi

echo "== preflight complete =="
