#!/usr/bin/env bash
# Shared .env.local loader. Source this (do not execute it) after cd-ing to the
# repository root:  . scripts/robot-demo/env-local.sh
#
# Precedence: the environment the operator brought to the stage WINS.
# .env.local only supplies values that are not already set, so exporting
# ROBOT_DEMO_BACKEND=robosuite (or REQUIRE_IB=1, or a pinned revision) before
# launching a script can no longer be silently overwritten by a developer
# machine's local file. A value that .env.local wanted to change is reported on
# stderr instead of being applied in silence.
#
# The file is still sourced as shell (comments, quoting and expansions behave
# exactly as before), just inside a child shell whose resulting environment is
# filtered before anything reaches this process.

robot_demo_load_env_local() {
  local file="${1:-.env.local}"
  [ -f "$file" ] || return 0

  local dump name value
  dump="$(mktemp "${TMPDIR:-/tmp}/env-local.XXXXXX")" || {
    echo "FAIL could not create a temporary file while loading $file" >&2
    return 1
  }
  # The child script below is deliberately unexpanded: it runs in the child.
  # shellcheck disable=SC2016
  if ! "${BASH:-bash}" -c '
        set -euo pipefail
        set -a
        # shellcheck disable=SC1090
        . "$1"
        set +a
        for name in $(compgen -e); do printf "%s=%s\0" "$name" "${!name}"; done
      ' robot-demo-env-local "$file" > "$dump"; then
    rm -f "$dump"
    echo "FAIL $file could not be sourced" >&2
    return 1
  fi

  while IFS= read -r -d '' entry; do
    name="${entry%%=*}"
    value="${entry#*=}"
    case "$name" in
      ''|_|PWD|OLDPWD|SHLVL|BASH_EXECUTION_STRING) continue ;;
    esac
    [[ "$name" =~ ^[A-Za-z_][A-Za-z0-9_]*$ ]] || continue
    if [ -z "${!name+set}" ]; then
      export "$name=$value"
    elif [ "${!name}" != "$value" ]; then
      echo "note keeping caller-provided $name; the value in $file is ignored" >&2
    fi
  done < "$dump"
  rm -f "$dump"
}

robot_demo_load_env_local "${ROBOT_DEMO_ENV_FILE:-.env.local}"
