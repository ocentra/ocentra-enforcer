const OPERATION_VALUES = new Set([
  "inspect",
  "edit",
  "commit",
  "push",
  "rebase",
  "merge",
  "pr_ready",
]);

const LOCK_KIND_VALUES = new Set([
  "writeLock",
  "globalWriteLock",
  "branchLease",
  "workReservation",
]);

const ON_CONFLICT_VALUES = new Set(["fail", "intent"]);

const LOCKFILE_NAMES = new Set([
  "cargo.lock",
  "package-lock.json",
  "pnpm-lock.yaml",
  "yarn.lock",
  "uv.lock",
  "poetry.lock",
]);

export function normalizeOperation(value, fallback = "commit") {
  const normalized = String(value ?? fallback).trim();
  if (!OPERATION_VALUES.has(normalized)) {
    throw new Error(
      `unsupported coordination operation: ${value}; expected ${[...OPERATION_VALUES].join(", ")}`,
    );
  }
  return normalized;
}

export function normalizeLockKind(value, fallback = "writeLock") {
  const normalized = String(value ?? fallback).trim();
  if (!LOCK_KIND_VALUES.has(normalized)) {
    throw new Error(
      `unsupported coordination lockKind: ${value}; expected ${[...LOCK_KIND_VALUES].join(", ")}`,
    );
  }
  return normalized;
}

export function normalizeOnConflict(value, fallback = "fail") {
  const normalized = String(value ?? fallback).trim();
  if (!ON_CONFLICT_VALUES.has(normalized)) {
    throw new Error(
      `unsupported coordination onConflict: ${value}; expected ${[...ON_CONFLICT_VALUES].join(", ")}`,
    );
  }
  return normalized;
}

export function normalizeCoordinationPath(value) {
  return String(value ?? "")
    .trim()
    .replace(/\\/gu, "/")
    .replace(/\/+/gu, "/")
    .replace(/^\.\//u, "")
    .toLowerCase();
}

export function buildClaimContext(args = {}, baseContext = {}) {
  const operation = normalizeOperation(args.operation, "edit");
  const lockKind = normalizeLockKind(args.lockKind, "writeLock");
  const onConflict = normalizeOnConflict(args.onConflict, "fail");
  return {
    ...baseContext,
    operation,
    lockKind,
    onConflict,
    ...(args.claimGroup ? { claimGroup: String(args.claimGroup) } : {}),
  };
}

export function buildRequestClaim({ writer, lane, paths, context, reason }) {
  return enrichClaim({
    writer,
    lane,
    paths,
    eventId: "__request__",
    ...(reason === undefined ? {} : { reason }),
    context,
  });
}

export function enrichClaim(claim) {
  const context = claim.context ?? {};
  const hasContext = claim.context !== undefined;
  const paths = (claim.paths ?? []).map(normalizeCoordinationPath).filter(Boolean);
  const declaredLockKind = normalizeLockKind(
    context.lockKind ?? claim.lockKind,
    hasContext ? "writeLock" : "globalWriteLock",
  );
  const singletonGroups = paths
    .map((entry) => protectedSingletonGroup(entry))
    .filter((entry) => entry !== null);
  const lockKind =
    declaredLockKind === "globalWriteLock" || singletonGroups.length > 0
      ? "globalWriteLock"
      : declaredLockKind;
  const operation = normalizeOperation(context.operation ?? claim.operation, "edit");
  const claimGroup = context.claimGroup ?? claim.claimGroup ?? null;
  const projectKey = normalizeKey(
    context.projectId ?? context.gitRemote ?? context.repoRoot ?? "legacy-unknown-project",
  );
  const worktreeKey = normalizeKey(
    context.worktreeRoot ?? context.repoRoot ?? "legacy-unknown-worktree",
  );
  const branchKey = normalizeKey(context.branch ?? "unknown-branch");
  const pathKeys = claimGroup === null ? paths : [normalizeKey(claimGroup)];
  const globalKeys =
    lockKind === "globalWriteLock"
      ? unique(
          singletonGroups.length > 0
            ? singletonGroups.map((group) => `${projectKey}:${group}`)
            : pathKeys.map((entry) => `${projectKey}:${entry}`),
        )
      : [];
  const physicalKeys =
    lockKind === "writeLock"
      ? pathKeys.map((entry) => `${projectKey}:${worktreeKey}:${entry}`)
      : [];
  const branchKeys =
    lockKind === "branchLease"
      ? [`${projectKey}:${branchKey}`]
      : pathKeys.map((entry) => `${projectKey}:${branchKey}:${entry}`);
  return {
    ...claim,
    paths,
    lockKind,
    operation,
    claimGroup,
    projectKey,
    worktreeKey,
    branchKey,
    pathKeys,
    globalKeys,
    physicalKeys,
    branchKeys,
    protectedSingleton: singletonGroups.length > 0,
  };
}

export function classifyOwnership(activeClaims, editIntents = []) {
  const claims = activeClaims.map(enrichClaim);
  const hardConflicts = [];
  const writeConflicts = [];
  const branchWriteConflicts = [];
  const globalWriteConflicts = [];
  const branchLeaseConflicts = [];
  const mergeRisks = [];
  const advisories = [];

  for (let leftIndex = 0; leftIndex < claims.length; leftIndex += 1) {
    for (let rightIndex = leftIndex + 1; rightIndex < claims.length; rightIndex += 1) {
      const left = claims[leftIndex];
      const right = claims[rightIndex];
      if (left === undefined || right === undefined || left.writer === right.writer) continue;
      const conflicts = classifyClaimPair(left, right);
      writeConflicts.push(...conflicts.writeConflicts);
      branchWriteConflicts.push(...conflicts.branchWriteConflicts);
      globalWriteConflicts.push(...conflicts.globalWriteConflicts);
      branchLeaseConflicts.push(...conflicts.branchLeaseConflicts);
      mergeRisks.push(...conflicts.mergeRisks);
      advisories.push(...conflicts.advisories);
    }
  }

  hardConflicts.push(
    ...writeConflicts,
    ...branchWriteConflicts,
    ...globalWriteConflicts,
    ...branchLeaseConflicts,
  );

  return {
    activeClaims: claims,
    writeLocks: claims.filter((claim) => claim.lockKind === "writeLock"),
    editIntents,
    hardConflicts,
    conflicts: hardConflicts,
    writeConflicts,
    branchWriteConflicts,
    mergeRisks,
    globalWriteConflicts,
    branchLeaseConflicts,
    advisories,
  };
}

export function blockersForRequest(activeClaims, requestClaim, operation) {
  const request = enrichClaim(requestClaim);
  const hardConflicts = [];
  const mergeRisks = [];
  const advisories = [];
  const effectiveOperation = normalizeOperation(operation ?? request.operation, "edit");

  for (const active of activeClaims.map(enrichClaim)) {
    if (active.writer === request.writer) continue;
    const conflicts = classifyClaimPair(active, request);
    const hard = [
      ...conflicts.writeConflicts,
      ...conflicts.branchWriteConflicts,
      ...conflicts.globalWriteConflicts,
      ...conflicts.branchLeaseConflicts,
    ];
    hardConflicts.push(...hard);
    mergeRisks.push(...conflicts.mergeRisks);
    advisories.push(...conflicts.advisories);
  }

  const blocksForOperation =
    effectiveOperation === "inspect"
      ? []
      : effectiveOperation === "push"
        ? hardConflicts.filter((conflict) => conflict.type === "branch-lease-conflict")
        : effectiveOperation === "pr_ready"
          ? [...hardConflicts, ...mergeRisks]
          : hardConflicts;

  return {
    request,
    operation: effectiveOperation,
    hardConflicts,
    mergeRisks,
    advisories,
    blockers: blocksForOperation,
  };
}

export function claimMatchesOperation(claim, path, operation, requestContext = {}) {
  const enriched = enrichClaim(claim);
  const normalizedPath = normalizeCoordinationPath(path);
  const requested = enrichClaim({
    writer: "__request__.lane",
    lane: "__request__",
    paths: [normalizedPath],
    eventId: "__request__",
    context: {
      ...requestContext,
      operation,
      lockKind: operation === "push" ? "branchLease" : "writeLock",
    },
  });
  if (operation === "commit") {
    return (
      enriched.lane === requestContext.lane &&
      sameProject(enriched, requested) &&
      sameWorktree(enriched, requested) &&
      overlapping(enriched.pathKeys, requested.pathKeys).length > 0
    );
  }
  return overlapping(enriched.pathKeys, requested.pathKeys).length > 0;
}

export function conflictTouchesPaths(conflict, changedPaths) {
  const normalized = changedPaths.map(normalizeCoordinationPath).filter(Boolean);
  return (conflict.paths ?? [])
    .map(normalizeCoordinationPath)
    .some((conflictPath) =>
      normalized.some((changedPath) => pathOverlaps(changedPath, conflictPath)),
    );
}

export function pathOverlaps(left, right) {
  return left === right || left.startsWith(`${right}/`) || right.startsWith(`${left}/`);
}

function classifyClaimPair(left, right) {
  const globalOverlap = overlapping(left.globalKeys, right.globalKeys);
  const physicalOverlap = overlapping(left.physicalKeys, right.physicalKeys);
  const branchLeaseOverlap =
    left.lockKind === "branchLease" && right.lockKind === "branchLease"
      ? overlapping(left.branchKeys, right.branchKeys)
      : [];
  const samePath = overlapping(left.pathKeys, right.pathKeys);
  const commonPaths = pathsForConflict(left, right, samePath);
  const result = {
    writeConflicts: [],
    branchWriteConflicts: [],
    globalWriteConflicts: [],
    branchLeaseConflicts: [],
    mergeRisks: [],
    advisories: [],
  };

  if (globalOverlap.length > 0) {
    result.globalWriteConflicts.push(conflict("global-write-conflict", left, right, commonPaths));
    return result;
  }
  if (physicalOverlap.length > 0) {
    result.writeConflicts.push(conflict("write-lock-conflict", left, right, commonPaths));
    return result;
  }
  if (branchLeaseOverlap.length > 0) {
    result.branchLeaseConflicts.push(conflict("branch-lease-conflict", left, right, commonPaths));
    return result;
  }
  if (samePath.length === 0 || !sameProject(left, right)) {
    return result;
  }
  if (sameBranch(left, right) && !sameWorktree(left, right)) {
    result.branchWriteConflicts.push(
      conflict("branch-write-conflict", left, right, commonPaths),
    );
    return result;
  }
  if (!sameBranch(left, right)) {
    result.mergeRisks.push(conflict("merge-risk", left, right, commonPaths));
    return result;
  }
  if (left.lockKind === "workReservation" || right.lockKind === "workReservation") {
    result.advisories.push(conflict("work-reservation-overlap", left, right, commonPaths));
  }
  return result;
}

function conflict(type, left, right, paths) {
  return {
    type,
    paths,
    lanes: [left.lane, right.lane],
    writers: [left.writer, right.writer],
    eventIds: [left.eventId, right.eventId],
    lockKinds: [left.lockKind, right.lockKind],
    branches: [left.branchKey, right.branchKey],
    worktrees: [left.worktreeKey, right.worktreeKey],
    projects: [left.projectKey, right.projectKey],
    owners: [ownerSummary(left), ownerSummary(right)],
  };
}

function ownerSummary(claim) {
  return {
    writer: claim.writer,
    lane: claim.lane,
    eventId: claim.eventId,
    lockKind: claim.lockKind,
    branch: claim.branchKey,
    worktree: claim.worktreeKey,
    paths: claim.paths,
    reason: claim.reason ?? null,
  };
}

function pathsForConflict(left, right, pathKeys) {
  const normalized = new Set(pathKeys);
  const paths = [...left.paths, ...right.paths].filter((entry) => {
    if (left.claimGroup !== null || right.claimGroup !== null) return true;
    return normalized.has(entry);
  });
  return unique(paths.length > 0 ? paths : [...left.paths, ...right.paths]);
}

function protectedSingletonGroup(path) {
  const normalized = normalizeCoordinationPath(path);
  const basename = normalized.split("/").at(-1) ?? normalized;
  if (LOCKFILE_NAMES.has(basename)) return `lockfile:${basename}`;
  if (/^(changelog|changes|release-notes)(\.md)?$/u.test(basename)) {
    return `release:${basename}`;
  }
  if (/^(version|VERSION)$/u.test(path)) return `release:${basename.toLowerCase()}`;
  if (normalized.includes("/migrations/") || normalized.startsWith("migrations/")) {
    return `migrations:${normalized}`;
  }
  if (
    normalized.includes("/generated/") ||
    normalized.startsWith("generated/") ||
    normalized.includes("generated") && /schema|contract|dto|bridge/u.test(normalized)
  ) {
    return `generated:${normalized}`;
  }
  if (normalized.startsWith(".github/workflows/")) {
    return `ci:${normalized}`;
  }
  return null;
}

function sameProject(left, right) {
  return left.projectKey === right.projectKey;
}

function sameWorktree(left, right) {
  return left.worktreeKey === right.worktreeKey;
}

function sameBranch(left, right) {
  return left.branchKey === right.branchKey;
}

function overlapping(left, right) {
  const rightSet = new Set(right);
  return unique(left.filter((entry) => rightSet.has(entry)));
}

function normalizeKey(value) {
  return String(value ?? "")
    .trim()
    .replace(/\\/gu, "/")
    .replace(/\/+/gu, "/")
    .toLowerCase();
}

function unique(values) {
  return [...new Set(values.filter(Boolean))];
}
