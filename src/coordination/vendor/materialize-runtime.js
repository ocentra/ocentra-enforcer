export function uniqueEvents(events, baseSeenEventIds) {
    const seenEventIds = new Set(baseSeenEventIds ?? []);
    const unique = [];
    let duplicateCount = 0;
    for (const event of events) {
        if (seenEventIds.has(event.id)) {
            duplicateCount += 1;
        }
        else {
            seenEventIds.add(event.id);
            unique.push(event);
        }
    }
    return { events: unique, duplicateCount, seenEventIds: [...seenEventIds] };
}

export function refreshTemporalState(lanes, workers, sessions) {
    const now = Date.now();
    for (const lane of lanes.values()) {
        if (lane.heartbeat !== undefined) {
            lane.heartbeat = { ...lane.heartbeat, stale: expired(lane.heartbeat.expiresAt, now) };
        }
    }
    for (const worker of workers.values()) {
        if (worker.heartbeat !== undefined) {
            worker.heartbeat = { ...worker.heartbeat, stale: expired(worker.heartbeat.expiresAt, now) };
        }
    }
    for (const [lane, session] of sessions) {
        sessions.set(lane, { ...session, stale: expired(session.expiresAt, now) });
    }
}

export function nextOrderCursor(baseCursor, events) {
    return events.reduce((cursor, event) => cursor === undefined || compareOrder(cursor, event) < 0
        ? { ts: event.ts, id: event.id }
        : cursor, baseCursor);
}

function expired(expiresAt, now) {
    return Date.parse(expiresAt) < now;
}

function compareOrder(left, right) {
    const ts = left.ts.localeCompare(right.ts);
    return ts === 0 ? left.id.localeCompare(right.id) : ts;
}
