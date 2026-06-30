import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
const PACK_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..", "..");
export function resolveLedgerRoot(env = process.env) {
    const hub = env.OCENTRA_COORDINATION_HUB ?? env.OCENTRA_ENFORCER_HUB ?? "ocentra-parent";
    if (env.LEDGER_ROOT ?? env.OCENTRA_COORDINATION_ROOT) {
        return resolve(env.LEDGER_ROOT ?? env.OCENTRA_COORDINATION_ROOT);
    }
    return resolve(resolveLedgerHome(env), hub);
}

export function resolveLedgerHome(env = process.env) {
    return resolve(env.OCENTRA_LEDGER_HOME ?? env.OCENTRA_COORDINATION_HOME ?? env.LEDGER_HOME ?? join(PACK_ROOT, ".ledger"));
}
