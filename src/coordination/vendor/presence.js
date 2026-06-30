export function buildPresenceMatrix(root, state, options = {}) {
    const rows = [...state.workers.values()].map((worker) => {
        const lane = state.lanes.get(worker.lane);
        const activeTask = worker.currentTaskId === undefined ? null : state.tasks.get(worker.currentTaskId) ?? null;
        const context = worker.context ?? {};
        return {
            writer: worker.writer,
            nodeId: worker.nodeId,
            nodeName: worker.nodeName,
            machine: context.machine ?? worker.nodeName,
            user: context.user ?? "unknown",
            os: context.os ?? "unknown",
            hub: context.hub ?? "unknown",
            projectId: context.projectId ?? "unknown",
            repoRoot: context.repoRoot ?? null,
            worktreeRoot: context.worktreeRoot ?? null,
            gitRemote: context.gitRemote ?? null,
            branch: context.branch ?? null,
            commit: context.commit ?? null,
            cwd: context.cwd ?? null,
            lane: worker.lane,
            codexThreadId: context.codexThreadId ?? "unknown",
            codexSessionId: context.codexSessionId ?? "unknown",
            pid: context.pid ?? null,
            state: worker.state,
            lastSeenAt: worker.lastSeenAt,
            heartbeatExpiresAt: worker.heartbeat?.expiresAt ?? null,
            activeTask,
            activeClaims: worker.activeClaims ?? [],
            unreadInboxCount: lane?.inbox.filter((item) => item.ackedBy.length === 0).length ?? 0,
            peerUrl: context.peerUrl ?? null,
            syncStatus: context.syncStatus ?? "unknown",
            stale: worker.state === "offline" || worker.heartbeat?.stale === true,
        };
    });
    const limit = Number.isFinite(options.limit) ? options.limit : rows.length;
    const limitedRows = rows.slice(0, limit);
    return {
        ok: true,
        root,
        generatedAt: new Date().toISOString(),
        totalRows: rows.length,
        rows: limitedRows,
        views: {
            byPc: groupRows(rows, (row) => row.machine),
            byProject: groupRows(rows, (row) => row.projectId),
            byWorktree: groupRows(rows, (row) => row.worktreeRoot ?? "unknown"),
            byLane: groupRows(rows, (row) => row.lane),
            byThread: groupRows(rows, (row) => row.codexThreadId),
            byClaimedPath: claimedPathView(rows),
            staleOffline: rows.filter((row) => row.stale).map(compactRow),
        },
    };
}

function groupRows(rows, keyForRow) {
    const grouped = {};
    for (const row of rows) {
        const key = keyForRow(row);
        grouped[key] ??= [];
        grouped[key].push(compactRow(row));
    }
    return grouped;
}

function claimedPathView(rows) {
    const claimed = {};
    for (const row of rows) {
        for (const claim of row.activeClaims) {
            for (const path of claim.paths ?? []) {
                claimed[path] ??= [];
                claimed[path].push({
                    ...compactRow(row),
                    eventId: claim.eventId,
                    reason: claim.reason ?? null,
                });
            }
        }
    }
    return claimed;
}

function compactRow(row) {
    return {
        writer: row.writer,
        lane: row.lane,
        state: row.state,
        nodeId: row.nodeId,
        nodeName: row.nodeName,
        machine: row.machine,
        projectId: row.projectId,
        worktreeRoot: row.worktreeRoot,
        branch: row.branch,
        commit: row.commit,
        codexThreadId: row.codexThreadId,
        codexSessionId: row.codexSessionId,
        lastSeenAt: row.lastSeenAt,
        heartbeatExpiresAt: row.heartbeatExpiresAt,
        activeClaimCount: row.activeClaims.length,
        unreadInboxCount: row.unreadInboxCount,
    };
}
