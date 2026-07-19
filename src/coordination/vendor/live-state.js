import { materialize, materializeEvents } from "./materialize.js";
import { readLiveDeltas } from "./live-deltas.js";
import { loadLiveIndex } from "./live-index.js";

/** Materialize coordination state from a trusted checkpoint plus verified live deltas. */
export async function materializeLive(root) {
    const index = await loadLiveIndex(root);
    if (index === null) {
        return materialize(root);
    }
    const cursor = index.orderCursor ?? index.state.orderCursor;
    const deltas = await readLiveDeltas(root, index.liveStreams);
    if (deltas === null || deltas.events.some((event) => compareOrder(event, cursor) < 0)) {
        return materialize(root);
    }
    const state = materializeEvents(deltas.events, {
        baseState: index.state,
        baseEventCount: index.state.dashboard?.eventCount ?? index.dashboard?.eventCount ?? 0,
        duplicateCount: index.state.dashboard?.duplicateCount ?? index.dashboard?.duplicateCount ?? 0,
        baseSeenEventIds: index.state.seenEventIds,
        baseOrderCursor: cursor,
        warnings: index.state.warnings ?? [],
    });
    return { ...state, checkpointAudit: index.audit ?? { ok: true, diagnostics: [] } };
}

function compareOrder(event, cursor) {
    const ts = event.ts.localeCompare(cursor.ts);
    return ts === 0 ? event.id.localeCompare(cursor.id) : ts;
}
