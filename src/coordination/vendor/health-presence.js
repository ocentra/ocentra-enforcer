import { pathOverlaps } from "./lock-policy.js";

export function buildHealthPresence(root, state, options = {}) {
  const rows = [...state.workers.values()].flatMap((worker) =>
    healthPresenceRows(worker, state),
  );
  const limit = Number.isFinite(options.limit) ? options.limit : 25;
  return {
    ok: true,
    root,
    generatedAt: new Date().toISOString(),
    totalRows: rows.length,
    rows: rows.slice(0, limit),
    views: {
      byClaimedPath: focusedClaimedPathView(rows, options.changedPaths ?? [], limit),
      staleOffline: rows.filter((row) => row.stale).slice(0, limit),
    },
  };
}

function healthPresenceRows(worker, state) {
  const lane = state.lanes.get(worker.lane);
  const groups =
    (worker.activeClaims ?? []).length === 0
      ? [[worker.context ?? {}, []]]
      : claimContextGroups(worker);
  return groups.map(([context, activeClaims]) => {
    return {
      writer: worker.writer,
      lane: worker.lane,
      state: worker.state,
      nodeId: worker.nodeId,
      nodeName: worker.nodeName,
      machine: context.machine ?? worker.nodeName,
      projectId: context.projectId ?? "unknown",
      worktreeRoot: context.worktreeRoot ?? null,
      branch: context.branch ?? null,
      commit: context.commit ?? null,
      codexThreadId: context.codexThreadId ?? "unknown",
      codexSessionId: context.codexSessionId ?? "unknown",
      lastSeenAt: worker.lastSeenAt,
      heartbeatExpiresAt: worker.heartbeat?.expiresAt ?? null,
      activeClaimCount: activeClaims.length,
      activeClaims,
      unreadInboxCount:
        lane?.inbox.filter((item) => item.ackedBy.length === 0).length ?? 0,
      stale: worker.state === "offline" || worker.heartbeat?.stale === true,
    };
  });
}

function focusedClaimedPathView(rows, changedPaths, limit) {
  if (changedPaths.length === 0) return {};
  const claimed = {};
  for (const row of rows) {
    appendFocusedClaims(claimed, row, changedPaths, limit);
  }
  return claimed;
}

function appendFocusedClaims(claimed, row, changedPaths, limit) {
  for (const claim of row.activeClaims ?? []) {
    for (const claimPath of claim.paths ?? []) {
      const normalizedClaimPath = normalizePath(claimPath);
      if (!changedPaths.some((changedPath) => pathOverlaps(changedPath, normalizedClaimPath))) {
        continue;
      }
      claimed[claimPath] ??= [];
      if (claimed[claimPath].length < limit) {
        claimed[claimPath].push(claimedPathEntry(row, claim));
      }
    }
  }
}

function claimedPathEntry(row, claim) {
  return {
    writer: row.writer,
    lane: row.lane,
    state: row.state,
    projectId: row.projectId,
    worktreeRoot: row.worktreeRoot,
    branch: row.branch,
    codexThreadId: row.codexThreadId,
    codexSessionId: row.codexSessionId,
    eventId: claim.eventId,
    reason: claim.reason ?? null,
  };
}

function claimContextGroups(worker) {
  const grouped = new Map();
  for (const claim of worker.activeClaims ?? []) {
    const context = { ...(worker.context ?? {}), ...(claim.context ?? {}) };
    const key = contextKey(context);
    const entry = grouped.get(key) ?? [context, []];
    entry[1].push(claim);
    grouped.set(key, entry);
  }
  return [...grouped.values()];
}

function contextKey(context) {
  return [
    context.projectId ?? "unknown",
    context.worktreeRoot ?? context.repoRoot ?? "unknown",
    context.branch ?? "unknown",
    context.codexThreadId ?? context.codexSessionId ?? "unknown",
  ].map((entry) => normalizePath(entry)).join("|");
}

function normalizePath(value) {
  return String(value).replace(/\\/gu, "/").replace(/^\.\//u, "").toLowerCase();
}
