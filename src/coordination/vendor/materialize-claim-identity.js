import { enrichClaim, normalizeCoordinationPath, pathOverlaps } from "./lock-policy.js";

export function claimIdentityKey(claim) {
  const enriched = enrichClaim(claim);
  return [
    enriched.writer,
    enriched.ownerKey,
    enriched.projectKey,
    enriched.worktreeKey,
    enriched.branchKey,
    enriched.lockKind,
    enriched.pathKeys.join("|"),
    enriched.claimGroup ?? "",
  ].join(":");
}

export function applyReleaseEvent(activeClaims, event, overlappingPaths) {
  for (const [key, claim] of activeClaims) {
    if (releaseEventMatchesClaim(event, claim, overlappingPaths)) {
      activeClaims.delete(key);
    }
  }
}

/** Return the active claims that replaying one release event would remove. */
export function claimsReleasedByEvent(activeClaims, event) {
  return activeClaims.filter((claim) => releaseEventMatchesClaim(event, claim));
}

function releaseEventMatchesClaim(event, activeClaim, overlappingPaths = defaultOverlappingPaths) {
  return (event.paths ?? []).some((path) =>
    releaseMatchesClaim(
      {
        writer: event.writer,
        lane: event.lane,
        paths: [path],
        eventId: event.id,
        ...(event.context === undefined ? {} : { context: event.context }),
      },
      activeClaim,
      overlappingPaths,
    ),
  );
}

function releaseMatchesClaim(releaseClaim, activeClaim, overlappingPaths) {
  const release = enrichClaim(releaseClaim);
  const active = enrichClaim(activeClaim);
  if (release.writer !== active.writer) return false;
  if (overlappingPaths(release.paths, active.paths).length === 0) return false;
  const context = releaseClaim.context ?? {};
  if (context.explicitReleaseScope !== true) return true;
  if (activeClaim.context === undefined) return true;
  const releaseHasExplicitOwner = hasMeaningfulContext(context, "codexThreadId", "codexSessionId");
  const activeHasExplicitOwner = hasMeaningfulContext(
    activeClaim.context ?? {},
    "codexThreadId",
    "codexSessionId",
  );
  if (
    releaseHasExplicitOwner !== activeHasExplicitOwner ||
    (releaseHasExplicitOwner && release.ownerKey !== active.ownerKey)
  ) {
    return false;
  }
  if (
    (hasMeaningfulContext(context, "projectId", "repoRoot", "gitRemote") &&
      !sameReleaseProject(release, active)) ||
    (hasMeaningfulContext(context, "worktreeRoot", "repoRoot") &&
      release.worktreeKey !== active.worktreeKey) ||
    (hasMeaningfulContext(context, "branch") && release.branchKey !== active.branchKey)
  ) {
    return false;
  }
  return true;
}

function defaultOverlappingPaths(left, right) {
  const normalizedRight = right.map(normalizeCoordinationPath);
  return left
    .map(normalizeCoordinationPath)
    .filter((leftPath) => normalizedRight.some((rightPath) => pathOverlaps(leftPath, rightPath)));
}

function sameReleaseProject(release, active) {
  const bothDeclareProject =
    hasMeaningfulContext(release.context ?? {}, "explicitProjectId") &&
    hasMeaningfulContext(active.context ?? {}, "explicitProjectId");
  if (bothDeclareProject) return release.projectKey === active.projectKey;
  return (
    release.projectKey === active.projectKey ||
    (release.gitRemoteKey !== null && release.gitRemoteKey === active.gitRemoteKey) ||
    (release.repoRootKey !== null && release.repoRootKey === active.repoRootKey)
  );
}

function hasMeaningfulContext(context, ...keys) {
  return keys.some((key) => {
    const value = context[key];
    return value !== undefined && value !== null && String(value).trim() !== "" && value !== "unknown";
  });
}
