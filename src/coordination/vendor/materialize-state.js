import { normalizeCoordinationPath } from "./lock-policy.js";
import { claimIdentityKey } from "./materialize-claim-identity.js";

/** Create the empty mutable coordination state used by the event materializer. */
export function emptyMaterializedState() {
    return {
        lanes: new Map(),
        writers: new Map(),
        workers: new Map(),
        tasks: new Map(),
        acks: new Map(),
        activeClaims: new Map(),
        editIntents: new Map(),
        sessions: new Map(),
    };
}

/** Rehydrate a persisted checkpoint into the mutable maps used by live materialization. */
export function thawMaterializedState(state) {
    const lanes = new Map(Object.entries(state.lanes ?? {}).map(([laneId, lane]) => [laneId, thawLane(lane)]));
    const workers = new Map(Object.entries(state.workers ?? {}));
    const acks = acksFromLanes(lanes);
    return {
        lanes,
        writers: writersFromWorkers(workers),
        workers,
        tasks: new Map(Object.entries(state.tasks ?? {})),
        acks,
        activeClaims: new Map((state.ownership?.activeClaims ?? []).map((claim) => [claimIdentityKey(claim), claim])),
        editIntents: new Map((state.ownership?.editIntents ?? []).map((intent) => [intentKey(intent), intent])),
        sessions: new Map(Object.entries(state.sessions ?? {})),
    };
}

function thawLane(lane) {
    return {
        ...lane,
        registeredWriters: new Set(lane.registeredWriters ?? []),
        inbox: [...(lane.inbox ?? [])],
        ackedMessageIds: [...(lane.ackedMessageIds ?? [])],
    };
}

function writersFromWorkers(workers) {
    return new Map([...workers.values()].map((worker) => [worker.writer, {
            writer: worker.writer,
            nodeId: worker.nodeId,
            nodeName: worker.nodeName,
            lane: worker.lane,
            ...(worker.context === undefined ? {} : { context: worker.context }),
        }]));
}

function acksFromLanes(lanes) {
    const acks = new Map();
    for (const lane of lanes.values()) {
        for (const item of lane.inbox) {
            const ackedBy = acks.get(item.id) ?? new Set();
            for (const writer of item.ackedBy ?? []) {
                ackedBy.add(writer);
            }
            acks.set(item.id, ackedBy);
        }
    }
    return acks;
}

function intentKey(intent) {
    return String(intent.writer) + ":" + intent.paths.map(normalizeCoordinationPath).join(",");
}
