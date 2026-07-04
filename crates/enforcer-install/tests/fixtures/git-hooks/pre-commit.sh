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

run_enforcer scan --root "$ROOT" --workspace --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"

if [ "${OCENTRA_ENFORCER_LANGUAGES:-}" != "" ]; then
  run_enforcer scan --root "$ROOT" --workspace --languages "$OCENTRA_ENFORCER_LANGUAGES" --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
fi

if [ "${OCENTRA_ENFORCER_CARGO:-0}" = "1" ]; then
  run_enforcer cargo --root "$ROOT" --workspace --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
fi

printf '%s\n' "ocentra-enforcer pre-commit completed for profile ${PROFILE}"
