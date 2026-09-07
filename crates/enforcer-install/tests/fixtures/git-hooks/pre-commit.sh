#!/bin/sh
set -eu

ROOT="${OCENTRA_ENFORCER_TARGET_ROOT:-$(pwd)}"
PROFILE="${OCENTRA_ENFORCER_PROFILE:-strict}"
if [ "${OCENTRA_ENFORCER_BIN:-}" != "" ]; then
  ENFORCER_MODE="bin"
elif [ -f "$ROOT/scripts/rust-rules.mjs" ]; then
  ENFORCER_MODE="local"
else
  ENFORCER_MODE="installed"
fi

run_enforcer() {
  if [ "$ENFORCER_MODE" = "local" ]; then
    node "$ROOT/scripts/rust-rules.mjs" "$@"
  elif [ "$ENFORCER_MODE" = "bin" ]; then
    "$OCENTRA_ENFORCER_BIN" "$@"
  else
    ocentra-enforcer "$@"
  fi
}

run_scoped_scan() {
  if [ "${OCENTRA_ENFORCER_PRECOMMIT_SCOPE:-staged}" = "workspace" ]; then
    run_enforcer scan --root "$ROOT" --all --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
    return
  fi

  set --
  while IFS= read -r path; do
    [ "$path" = "" ] && continue
    set -- "$@" "$path"
  done <<EOF
$(git -C "$ROOT" diff --cached --name-only --diff-filter=ACMR)
EOF

  if [ "$#" -eq 0 ]; then
    printf '%s\n' "ocentra-enforcer pre-commit skipped: no staged files"
    return
  fi

  run_enforcer scan --root "$ROOT" "$@" --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
}

if [ "$ENFORCER_MODE" = "local" ]; then
  node "$ROOT/scripts/precommit-ratchet.mjs" "$ROOT" "$ROOT"
fi
run_scoped_scan

if [ "${OCENTRA_ENFORCER_LANGUAGES:-}" != "" ]; then
  run_enforcer scan --root "$ROOT" --all --languages "$OCENTRA_ENFORCER_LANGUAGES" --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
fi

if [ "${OCENTRA_ENFORCER_CARGO:-0}" = "1" ]; then
  run_enforcer cargo --root "$ROOT" --all --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
fi

printf '%s\n' "ocentra-enforcer pre-commit completed for profile ${PROFILE}"
