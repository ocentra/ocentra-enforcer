#!/bin/sh
set -eu

ROOT="${OCENTRA_ENFORCER_TARGET_ROOT:-$(pwd)}"
PROFILE="${OCENTRA_ENFORCER_PROFILE:-strict}"
ENFORCER="${OCENTRA_ENFORCER_BIN:-ocentra-enforcer}"

"$ENFORCER" scan --root "$ROOT" --workspace --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"

if [ "${OCENTRA_ENFORCER_CARGO:-0}" = "1" ]; then
  "$ENFORCER" cargo --root "$ROOT" --workspace --config "${OCENTRA_ENFORCER_CONFIG:-ocentra-enforcer.config.json}"
fi

printf '%s\n' "ocentra-enforcer pre-commit completed for profile ${PROFILE}"
