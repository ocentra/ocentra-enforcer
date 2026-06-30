import path from "node:path";
import { stat } from "node:fs/promises";
import { appendEvent } from "./vendor/stream.js";
import {
  getActiveTasks,
  getWorkers,
  materialize,
  materializedToJson,
} from "./vendor/materialize.js";
import { ensureDaemon } from "./vendor/daemon.js";
import { inspectLedger } from "./vendor/doctor.js";
import { guardLedger } from "./vendor/guard.js";
import { initIdentity, loadIdentity, resolveLane } from "./vendor/identity.js";
import { resolveLedgerRoot } from "./vendor/root.js";
import { normalizeClaimPaths } from "./vendor/claim-policy.js";
import { buildCoordinationContext } from "./vendor/context.js";
import { buildStreamManifest } from "./vendor/manifest.js";
import { notify } from "./vendor/notify.js";
import { addPeer, loadPeerRegistry, removePeer, resolvePeer } from "./vendor/peers.js";
import { buildPresenceMatrix } from "./vendor/presence.js";
import { rebuildCoordinationIndex } from "./vendor/read-index.js";
import {
  repairLegacyHashCompatibility,
  repairSequenceBreaks,
} from "./vendor/repair.js";
import { compactLedger } from "./vendor/retention.js";
import { syncFromHttpPeer } from "./vendor/sync/http.js";
import { syncFromPeer } from "./vendor/sync/local.js";
import {
  parseClaimPath,
  parseEventId,
  parseLaneId,
  parseMessageAddress,
  parseTaskId,
  parseTaskState,
  parseUserText,
  parseWorkerState,
  parseWriterId,
  writerId,
} from "./vendor/domain.js";
import {
  blockersForRequest,
  buildClaimContext,
  buildRequestClaim,
  normalizeOnConflict,
  normalizeOperation,
} from "./vendor/lock-policy.js";

export function coordinationRoot(args = {}) {
  return resolveLedgerRoot({
    ...process.env,
    ...(args.stateRoot ? { LEDGER_ROOT: args.stateRoot } : {}),
    ...(args.hub ? { OCENTRA_COORDINATION_HUB: args.hub } : {}),
  });
}

export async function coordinationInit(args = {}) {
  const root = coordinationRoot(args);
  const hub = args.hub ?? "ocentra-parent";
  const lane = args.lane ?? "primary";
  return initIdentity({ root, hub, lane });
}

export async function coordinationStatus(args = {}) {
  const root = coordinationRoot(args);
  const state = await materialize(root);
  return {
    ok: true,
    root,
    state: materializedToJson(state),
  };
}

export async function coordinationHealth(args = {}) {
  const root = coordinationRoot(args);
  const inspection = await inspectLedger(root);
  const changedPaths = normalizeHealthPaths(args);
  const focused = changedPaths.length > 0 && args.focused !== false;
  let state;
  try {
    state = await materialize(root);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      ok: false,
      root,
      canInspect: false,
      canLockPaths: false,
      canWriteClaimedPaths: false,
      mustWait: true,
      mustRepairLedger: true,
      diagnostics: compactDiagnostics([
        ...inspection.diagnostics,
        {
          level: "error",
          message,
        },
      ], args.limit),
      warnings: [],
      conflicts: [],
      conflictCount: 0,
      globalConflictCount: 0,
      focused,
      changedPaths,
      staleSessions: [],
      guard: {
        ok: false,
        error: message,
      },
      dashboard: null,
      presence: null,
      nextStep:
        "Repair stream hash/sequence issues first: run coordination repair all as a dry-run, then rerun with --write after reviewing backups.",
    };
  }
  const conflicts = state.ownership.hardConflicts ?? state.ownership.conflicts;
  const blockingConflicts = focused
    ? conflicts.filter((conflict) => conflictTouchesPaths(conflict, changedPaths))
    : conflicts;
  const warnings = state.warnings;
  const guard = await tryGuard(root, args);
  const corruptDiagnostics = inspection.diagnostics.filter(
    (diagnostic) =>
      /hash|sequence|previous|pointer|malformed|corrupt|lock/iu.test(
        diagnostic.message ?? JSON.stringify(diagnostic),
      ),
  );
  const staleSessions = [...state.sessions.values()].filter((session) =>
    session.expiresAt ? Date.parse(session.expiresAt) < Date.now() : false,
  );
  const mustRepairLedger =
    !inspection.ok || corruptDiagnostics.length > 0 || warnings.length > 0;
  const pathLockDenied = guard.result?.ok === false;
  return {
    ok: !mustRepairLedger && !pathLockDenied && blockingConflicts.length === 0,
    root,
    canInspect: inspection.ok,
    canLockPaths: !mustRepairLedger && (guard.result?.blockers?.length ?? blockingConflicts.length) === 0,
    canWriteClaimedPaths: guard.result?.ok ?? !mustRepairLedger,
    mustWait: pathLockDenied || blockingConflicts.length > 0,
    mustRepairLedger,
    diagnostics: compactDiagnostics(inspection.diagnostics, args.limit),
    warnings,
    conflicts: compactConflicts(blockingConflicts, args.limit),
    conflictCount: blockingConflicts.length,
    globalConflictCount: conflicts.length,
    hardConflicts: compactConflicts(blockingConflicts, args.limit),
    hardConflictCount: blockingConflicts.length,
    branchWriteConflicts: compactConflicts(state.ownership.branchWriteConflicts ?? [], args.limit),
    branchWriteConflictCount: state.ownership.branchWriteConflicts?.length ?? 0,
    mergeRisks: compactConflicts(state.ownership.mergeRisks ?? [], args.limit),
    mergeRiskCount: state.ownership.mergeRisks?.length ?? 0,
    globalWriteConflicts: compactConflicts(state.ownership.globalWriteConflicts ?? [], args.limit),
    globalWriteConflictCount: state.ownership.globalWriteConflicts?.length ?? 0,
    editIntents: (state.ownership.editIntents ?? []).slice(0, Number.isFinite(args.limit) ? args.limit : 25),
    editIntentCount: state.ownership.editIntents?.length ?? 0,
    focused,
    changedPaths,
    operation: normalizeOperation(args.operation, changedPaths.length > 0 ? "commit" : "inspect"),
    staleSessions,
    guard: guard.result,
    dashboard: state.dashboard,
    presence: buildHealthPresence(root, state, {
      changedPaths,
      limit: args.limit ?? 25,
    }),
  };
}

export async function coordinationPresence(args = {}) {
  const root = coordinationRoot(args);
  const state = await materialize(root);
  const presence = buildPresenceMatrix(root, state, { limit: args.limit });
  await rebuildCoordinationIndex(root, { limit: args.limit });
  return presence;
}

export async function coordinationInbox(args = {}) {
  const root = coordinationRoot(args);
  const lane = parseLaneId(args.lane ?? (await loadIdentity(root)).defaultLane);
  const state = await materialize(root);
  const inbox = state.lanes.get(lane)?.inbox ?? [];
  return {
    ok: true,
    root,
    lane,
    inbox: args.all ? inbox : inbox.filter((item) => item.ackedBy.length === 0),
  };
}

export async function coordinationAck(args = {}) {
  rejectUnexpectedAction(args, "ack", "coordination ack");
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const lane = resolveLane(config, args.lane);
  const event = await appendEvent(root, config, lane, {
    type: "ack",
    messageId: parseEventId(required(args.messageId, "messageId")),
    context: contextFor(args),
  });
  return { ok: true, root, event };
}

export async function coordinationMessage(args = {}) {
  rejectUnexpectedAction(args, ["message", "send"], "coordination message");
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const event = await appendEvent(root, config, config.defaultLane, {
    type: "message",
    to: parseMessageAddress(required(args.to ?? args.lane, "to")),
    body: parseUserText(required(args.body, "body")),
    context: contextFor(args),
  });
  return { ok: true, root, event };
}

export async function coordinationClaim(args = {}) {
  rejectUnexpectedAction(args, "claim", "coordination claim");
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const lane = parseLaneId(args.lane ?? config.defaultLane);
  const paths = await normalizeClaimPaths(
    path.resolve(args.root ?? process.cwd()),
    pathList(args.paths),
  );
  if (paths.length === 0) throw new Error("coordination claim requires paths");
  const baseContext = contextFor({ ...args, repoRoot: args.root, cwd: args.root });
  const claimContext = buildClaimContext(args, baseContext);
  const writer = writerId(config.nodeId, lane);
  const request = buildRequestClaim({
    writer,
    lane,
    paths,
    ...(args.reason ? { reason: parseUserText(args.reason) } : {}),
    context: claimContext,
  });
  const state = await materialize(root);
  const decision = blockersForRequest(state.ownership.activeClaims, request, claimContext.operation);
  if (decision.blockers.length > 0) {
    const onConflict = normalizeOnConflict(args.onConflict, claimContext.onConflict);
    const blockingOwners = blockingOwnersFor(decision.blockers);
    if (onConflict === "intent") {
      const intentEvent = await appendEvent(root, config, lane, {
        type: "editIntent",
        paths,
        ...(args.reason ? { reason: parseUserText(args.reason) } : {}),
        context: {
          ...claimContext,
          intentFor: "writeLock",
          blockingOwners,
          blockerCount: decision.blockers.length,
        },
      });
      return {
        ok: false,
        root,
        intentQueued: true,
        event: intentEvent,
        blockers: decision.blockers,
        blockingOwners,
        mergeRisks: decision.mergeRisks,
        nextStep:
          "Wait for a release notification, then re-read the file before claiming and editing.",
      };
    }
    return {
      ok: false,
      root,
      intentQueued: false,
      blockers: decision.blockers,
      blockingOwners,
      mergeRisks: decision.mergeRisks,
      nextStep:
        "The requested path is blocked by an active write/global/branch lock. Re-run with onConflict=intent to queue, or wait for release.",
    };
  }
  const event = await appendEvent(root, config, lane, {
    type: "claim",
    paths,
    ...(args.reason ? { reason: parseUserText(args.reason) } : {}),
    context: claimContext,
  });
  return { ok: true, root, event };
}

export async function coordinationRelease(args = {}) {
  rejectUnexpectedAction(args, "release", "coordination release");
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const lane = parseLaneId(args.lane ?? config.defaultLane);
  const paths = pathList(args.paths).map((entry) => parseClaimPath(entry));
  if (paths.length === 0) throw new Error("coordination release requires paths");
  const event = await appendEvent(root, config, lane, {
    type: "release",
    paths,
    ...(args.reason ? { reason: parseUserText(args.reason) } : {}),
    context: contextFor(args),
  });
  const state = await materialize(root);
  const notificationEvents = [];
  const notifiedLanes = new Set();
  for (const intent of nextEditIntentsForPaths(state.ownership.editIntents ?? [], paths)) {
    if (intent.lane === lane || notifiedLanes.has(intent.lane)) continue;
    notifiedLanes.add(intent.lane);
    notificationEvents.push(
      await appendEvent(root, config, lane, {
        type: "message",
        to: parseMessageAddress(intent.lane),
        body: parseUserText(
          `Released ${paths.join(", ")}. Re-read the file before claiming and editing; queued intent ${intent.eventId}.`,
        ),
        context: contextFor({
          ...args,
          releaseEventId: event.id,
          editIntentId: intent.eventId,
          notificationKind: "editIntentReleased",
        }),
      }),
    );
  }
  return { ok: true, root, event, notificationEvents };
}

export async function coordinationGuard(args = {}) {
  const root = coordinationRoot(args);
  const requestedPaths = pathList(args.paths);
  const changedPaths =
    requestedPaths.length > 0 ? requestedPaths : pathList(args.changedPaths);
  const result = await guardLedger(root, {
    lane: args.lane ?? (await loadIdentity(root)).defaultLane,
    changedPaths,
    root: args.root,
    repoRoot: args.repoRoot ?? args.root,
    worktreeRoot: args.worktreeRoot,
    cwd: args.cwd ?? args.root,
    projectId: args.projectId,
    gitRemote: args.gitRemote,
    branch: args.branch,
    commit: args.commit,
    codexThreadId: args.codexThreadId,
    codexSessionId: args.codexSessionId,
    operation: args.operation,
    lockKind: args.lockKind,
    claimGroup: args.claimGroup,
    allowMergeRisks: args.allowMergeRisks === true,
    focused: args.focused !== false,
    limit: args.limit,
    allowPrimaryWithoutClaims: args.allowPrimaryWithoutClaims === true,
    ...(args.sessionId ? { sessionId: args.sessionId } : {}),
  });
  return { ok: result.ok, root, result };
}

export async function coordinationReport(args = {}) {
  rejectUnexpectedAction(args, "report", "coordination report");
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const lane = resolveLane(config, args.lane);
  const event = await appendEvent(root, config, lane, {
    type: "report",
    summary: parseUserText(required(args.summary, "summary")),
    ...(args.taskId ? { taskId: parseTaskId(args.taskId) } : {}),
    context: contextFor(args),
  });
  return { ok: true, root, event };
}

export async function coordinationWorkers(args = {}) {
  const root = coordinationRoot(args);
  const state = await materialize(root);
  return { ok: true, root, workers: getWorkers(state) };
}

export async function coordinationTasks(args = {}) {
  const root = coordinationRoot(args);
  const state = await materialize(root);
  return { ok: true, root, tasks: getActiveTasks(state) };
}

export async function coordinationWorkerUpdate(args = {}) {
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const event = await appendEvent(root, config, parseLaneId(args.lane), {
    type: "worker.update",
    workerState: parseWorkerState(required(args.state, "state")),
    summary: parseUserText(required(args.summary, "summary")),
    ...(args.taskId ? { taskId: parseTaskId(args.taskId) } : {}),
    context: contextFor(args),
  });
  return { ok: true, root, event };
}

export async function coordinationTaskUpdate(args = {}) {
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const event = await appendEvent(root, config, parseLaneId(args.lane), {
    type: "task.update",
    taskId: parseTaskId(required(args.taskId, "taskId")),
    taskState: parseTaskState(required(args.state, "state")),
    summary: parseUserText(required(args.summary, "summary")),
    context: contextFor(args),
  });
  return { ok: true, root, event };
}

export async function coordinationStreams(args = {}) {
  const root = coordinationRoot(args);
  return buildStreamManifest(root);
}

export async function coordinationIndex(args = {}) {
  const root = coordinationRoot(args);
  return rebuildCoordinationIndex(root, { limit: args.limit });
}

export async function coordinationSync(args = {}) {
  const root = coordinationRoot(args);
  const peer = required(args.peer ?? args.url ?? args.peerUrl, "peer");
  const result = await syncPeer(root, peer, args);
  const index = await rebuildCoordinationIndex(root, { limit: args.limit });
  return {
    ok: result.conflicts.length === 0,
    root,
    peer,
    result,
    index,
  };
}

export async function coordinationPeer(args = {}) {
  const root = coordinationRoot(args);
  const action = args.action ?? args.state ?? "list";
  if (action === "add") {
    return {
      ok: true,
      root,
      registry: await addPeer(root, {
        name: required(args.name ?? args.peer, "name"),
        url: required(args.url ?? args.peerUrl, "url"),
        ...(args.tokenEnv ? { tokenEnv: args.tokenEnv } : {}),
        ...(args.mode ? { mode: args.mode } : {}),
      }),
    };
  }
  if (action === "remove") {
    return {
      ok: true,
      root,
      registry: await removePeer(root, required(args.name ?? args.peer, "name")),
    };
  }
  if (action === "health") {
    const peer = required(args.peer ?? args.name ?? args.url ?? args.peerUrl, "peer");
    const resolved = await resolvePeer(root, peer);
    const response = await fetch(new URL("/health", resolved.url), requestInit(args.token ?? resolved.token ?? process.env.LEDGER_PEER_TOKEN));
    return {
      ok: response.ok,
      root,
      peer,
      url: resolved.url,
      status: response.status,
      body: response.ok ? await response.json() : await response.text(),
    };
  }
  if (action === "sync") {
    return coordinationSync({ ...args, stateRoot: root, peer: args.peer ?? args.name ?? args.url ?? args.peerUrl });
  }
  return { ok: true, root, registry: await loadPeerRegistry(root) };
}

export async function coordinationEnsure(args = {}) {
  const root = coordinationRoot(args);
  const port = Number(args.port ?? process.env.LEDGER_PORT ?? 8787);
  const host = args.host ?? process.env.LEDGER_HOST ?? "127.0.0.1";
  const token = args.token ?? process.env.LEDGER_HTTP_TOKEN;
  return ensureDaemon({
    root,
    port,
    host,
    ...(token ? { token } : {}),
  });
}

export async function coordinationCompact(args = {}) {
  const root = coordinationRoot(args);
  const keepLatest = Number(args.keepLatest ?? 250);
  const result = await compactLedger(root, { keepLatest });
  const index = await rebuildCoordinationIndex(root, { limit: args.limit });
  return { ok: true, root, ...result, index };
}

export async function coordinationRepair(args = {}) {
  const root = coordinationRoot(args);
  const action = args.action ?? args.mode ?? "legacy-hash";
  const options = {
    dryRun: args.write === true ? false : args.dryRun !== false,
    limit: args.limit,
  };
  if (action === "legacy-hash") {
    return repairLegacyHashCompatibility(root, options);
  }
  if (action === "sequence" || action === "sequence-breaks") {
    return repairSequenceBreaks(root, options);
  }
  if (action === "all") {
    const legacyHash = await repairLegacyHashCompatibility(root, options);
    const sequence = await repairSequenceBreaks(root, options);
    return {
      ok: legacyHash.ok && sequence.ok,
      root,
      dryRun: options.dryRun,
      action,
      legacyHash,
      sequence,
      nextStep: options.dryRun
        ? "Review both repair reports, then re-run with CLI --write if the stream changes are expected."
        : "Run coordination doctor/health. If conflicts remain, use coordination repair stale-claims on exact paths.",
    };
  }
  if (["stale-claims", "claim-conflicts", "conflicts"].includes(action)) {
    return repairClaimConflicts(root, args, options);
  }
  throw new Error(`unsupported coordination repair action: ${action}`);
}

export async function coordinationNotify(args = {}) {
  const root = coordinationRoot(args);
  const config = await loadIdentity(root);
  const lane = parseLaneId(args.lane ?? config.defaultLane);
  return notify({
    lane,
    root,
    json: true,
    peek: args.peek === true,
    exitCode: false,
    ...(args.stateFile ? { stateFile: args.stateFile } : {}),
  });
}

export async function coordinationMail(args = {}) {
  const action = args.action ?? args.state ?? "inbox";
  if (action === "send") return coordinationMessage(args);
  if (action === "ack") return coordinationAck(args);
  return coordinationInbox(args);
}

async function tryGuard(root, args) {
  if (!args.lane && !args.paths && !args.changedPaths) return { result: null };
  try {
    return { result: (await coordinationGuard({ ...args, stateRoot: root })).result };
  } catch (error) {
    return {
      result: {
        ok: false,
        error: error instanceof Error ? error.message : String(error),
      },
    };
  }
}

function compactDiagnostics(diagnostics, limit = 25) {
  return diagnostics.slice(0, Number.isFinite(limit) ? limit : 25);
}

function compactConflicts(conflicts, limit = 25) {
  return conflicts.slice(0, Number.isFinite(limit) ? limit : 25);
}

function blockingOwnersFor(blockers) {
  return [
    ...new Map(
      blockers
        .flatMap((blocker) => blocker.owners ?? [])
        .filter((owner) => owner.eventId !== "__request__")
        .map((owner) => [owner.writer, owner]),
    ).values(),
  ];
}

function nextEditIntentsForPaths(editIntents, paths) {
  const normalized = normalizeRepoPaths(paths);
  return editIntents.filter((intent) =>
    (intent.paths ?? [])
      .map((entry) => entry.replace(/\\/gu, "/").replace(/^\.\//u, "").toLowerCase())
      .some((intentPath) =>
        normalized.some((releasedPath) => pathOverlaps(intentPath, releasedPath)),
      ),
  );
}

async function repairClaimConflicts(root, args, options) {
  const paths = pathList(args.paths).map((entry) => parseClaimPath(entry));
  const dryRun = options.dryRun !== false;
  const limit = Number.isFinite(options.limit) ? options.limit : 25;
  let state;
  try {
    state = await materialize(root);
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error);
    return {
      ok: false,
      root,
      dryRun,
      action: "stale-claims",
      paths,
      owner: args.owner ?? null,
      error: message,
      nextStep:
        "Repair stream hash/sequence issues first: run coordination repair all as a dry-run, then rerun with --write after reviewing backups.",
    };
  }
  const matchingConflicts = filterConflictsByPaths(state.ownership.conflicts, paths);
  const matchingClaims = filterClaimsByPaths(state.ownership.activeClaims, paths);
  const suggestedCommands = buildClaimRepairCommands(root, args, matchingConflicts, paths);
  if (dryRun) {
    return {
      ok: true,
      root,
      dryRun,
      action: "stale-claims",
      paths,
      owner: args.owner ?? null,
      matchingConflictCount: matchingConflicts.length,
      matchingClaimCount: matchingClaims.length,
      conflicts: compactConflicts(matchingConflicts, limit),
      claims: matchingClaims.slice(0, limit),
      suggestedCommands,
      nextStep:
        paths.length === 0
          ? "Pass --paths with exact stale/conflicting paths, then re-run with --write if the cleanup is expected."
          : "Re-run with --write to append a claim.resolve event for these exact paths.",
    };
  }
  if (paths.length === 0) {
    throw new Error("coordination repair stale-claims --write requires exact --paths");
  }
  const config = await loadIdentity(root);
  const lane = parseLaneId(args.lane ?? config.defaultLane);
  const owner = args.owner === undefined ? undefined : parseWriterId(args.owner);
  const event = await appendEvent(root, config, lane, {
    type: "claim.resolve",
    paths,
    ...(owner === undefined ? {} : { owner }),
    context: contextFor({
      ...args,
      repairAction: "stale-claims",
      matchingConflictCount: matchingConflicts.length,
      matchingClaimCount: matchingClaims.length,
    }),
  });
  const repairedState = await materialize(root);
  const index = await rebuildCoordinationIndex(root, { limit });
  const remainingConflicts = filterConflictsByPaths(repairedState.ownership.conflicts, paths);
  return {
    ok: remainingConflicts.length === 0,
    root,
    dryRun,
    action: "stale-claims",
    paths,
    owner: owner ?? null,
    event,
    resolvedClaimCount: matchingClaims.length,
    resolvedConflictCount: matchingConflicts.length,
    remainingConflictCount: remainingConflicts.length,
    remainingConflicts: compactConflicts(remainingConflicts, limit),
    index,
    nextStep: "Run coordination guard/health for the exact paths that were cleaned up.",
  };
}

function normalizeHealthPaths(args) {
  const paths = pathList(args.paths);
  return paths.length > 0 ? normalizeRepoPaths(paths) : normalizeRepoPaths(pathList(args.changedPaths));
}

function normalizeRepoPaths(paths) {
  return paths
    .map((entry) => entry.replace(/\\/gu, "/").replace(/^\.\//u, "").toLowerCase())
    .filter(Boolean);
}

function conflictTouchesPaths(conflict, changedPaths) {
  return conflict.paths
    .map((entry) => entry.replace(/\\/gu, "/").replace(/^\.\//u, "").toLowerCase())
    .some((conflictPath) =>
      changedPaths.some((changedPath) => pathOverlaps(changedPath, conflictPath)),
    );
}

function filterConflictsByPaths(conflicts, paths) {
  if (paths.length === 0) return conflicts;
  const normalizedPaths = normalizeRepoPaths(paths);
  return conflicts.filter((conflict) => conflictTouchesPaths(conflict, normalizedPaths));
}

function filterClaimsByPaths(claims, paths) {
  if (paths.length === 0) return claims;
  const normalizedPaths = normalizeRepoPaths(paths);
  return claims.filter((claim) =>
    (claim.paths ?? [])
      .map((entry) => entry.replace(/\\/gu, "/").replace(/^\.\//u, "").toLowerCase())
      .some((claimPath) =>
        normalizedPaths.some((repairPath) => pathOverlaps(claimPath, repairPath)),
      ),
  );
}

function buildClaimRepairCommands(root, args, conflicts, paths) {
  const selectedPaths = paths.length > 0
    ? paths
    : unique(conflicts.flatMap((conflict) => conflict.paths ?? []));
  if (selectedPaths.length === 0) return [];
  const stateRootArg = `--state-root ${quoteCli(root)}`;
  const hubArg = args.hub ? ` --hub ${quoteCli(args.hub)}` : "";
  const laneArg = args.lane ? ` --lane ${quoteCli(args.lane)}` : "";
  const ownerArg = args.owner ? ` --owner ${quoteCli(args.owner)}` : "";
  return [
    `node scripts/rust-rules.mjs coordination repair stale-claims ${stateRootArg}${hubArg}${laneArg} --paths ${quoteCli(selectedPaths.join(","))}${ownerArg} --json`,
    `node scripts/rust-rules.mjs coordination repair stale-claims ${stateRootArg}${hubArg}${laneArg} --paths ${quoteCli(selectedPaths.join(","))}${ownerArg} --write --json`,
  ];
}

function pathOverlaps(left, right) {
  return left === right || left.startsWith(`${right}/`) || right.startsWith(`${left}/`);
}

function unique(values) {
  return [...new Set(values)];
}

function quoteCli(value) {
  return `"${String(value).replaceAll('"', '\\"')}"`;
}

function buildHealthPresence(root, state, options = {}) {
  const rows = [...state.workers.values()].map((worker) => {
    const lane = state.lanes.get(worker.lane);
    const context = worker.context ?? {};
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
      activeClaimCount: (worker.activeClaims ?? []).length,
      unreadInboxCount:
        lane?.inbox.filter((item) => item.ackedBy.length === 0).length ?? 0,
      stale: worker.state === "offline" || worker.heartbeat?.stale === true,
    };
  });
  const limit = Number.isFinite(options.limit) ? options.limit : 25;
  return {
    ok: true,
    root,
    generatedAt: new Date().toISOString(),
    totalRows: rows.length,
    rows: rows.slice(0, limit),
    views: {
      byClaimedPath: focusedClaimedPathView(
        [...state.workers.values()],
        options.changedPaths ?? [],
        limit,
      ),
      staleOffline: rows.filter((row) => row.stale).slice(0, limit),
    },
  };
}

function focusedClaimedPathView(workers, changedPaths, limit) {
  if (changedPaths.length === 0) return {};
  const claimed = {};
  for (const worker of workers) {
    for (const claim of worker.activeClaims ?? []) {
      for (const claimPath of claim.paths ?? []) {
        const normalizedClaimPath = claimPath
          .replace(/\\/gu, "/")
          .replace(/^\.\//u, "")
          .toLowerCase();
        if (!changedPaths.some((changedPath) => pathOverlaps(changedPath, normalizedClaimPath))) {
          continue;
        }
        claimed[claimPath] ??= [];
        if (claimed[claimPath].length < limit) {
          claimed[claimPath].push({
            writer: worker.writer,
            lane: worker.lane,
            state: worker.state,
            eventId: claim.eventId,
            reason: claim.reason ?? null,
          });
        }
      }
    }
  }
  return claimed;
}

function pathList(value) {
  if (Array.isArray(value)) return value.map((entry) => String(entry));
  if (typeof value === "string") {
    return value
      .split(/[,\n]/u)
      .map((entry) => entry.trim())
      .filter(Boolean);
  }
  return [];
}

function contextFor(args = {}) {
  return buildCoordinationContext(args);
}

async function syncPeer(root, peer, args) {
  if (isHttpPeer(peer)) {
    return syncFromHttpPeer(root, peer, args.token ?? process.env.LEDGER_PEER_TOKEN);
  }
  if (isLocalPeerPath(peer) || (await pathExists(peer))) {
    return syncFromPeer(root, path.resolve(peer));
  }
  const resolved = await resolvePeer(root, peer);
  return syncFromHttpPeer(root, resolved.url, args.token ?? resolved.token ?? process.env.LEDGER_PEER_TOKEN);
}

function isHttpPeer(peer) {
  return peer.startsWith("http://") || peer.startsWith("https://");
}

function isLocalPeerPath(peer) {
  return (
    peer.includes(":\\") ||
    peer.includes(":/") ||
    peer.startsWith(".") ||
    peer.startsWith("/") ||
    peer.startsWith("\\") ||
    peer.includes("\\")
  );
}

async function pathExists(candidate) {
  try {
    await stat(path.resolve(candidate));
    return true;
  } catch {
    return false;
  }
}

function requestInit(token) {
  return token === undefined || token.length === 0
    ? undefined
    : { headers: { authorization: `Bearer ${token}` } };
}

function required(value, name) {
  if (typeof value !== "string" || value.length === 0) {
    throw new Error(`coordination ${name} is required`);
  }
  return value;
}

function rejectUnexpectedAction(args, expected, commandName) {
  const expectedActions = Array.isArray(expected) ? expected : [expected];
  if (args.action !== undefined && !expectedActions.includes(args.action)) {
    throw new Error(
      `${commandName} does not support action=${JSON.stringify(args.action)}; use the matching MCP tool instead.`,
    );
  }
}
