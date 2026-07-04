import { enrichClaim } from "./lock-policy.js";

export function claimIdentityKey(claim) {
  const enriched = enrichClaim(claim);
  return [
    enriched.writer,
    enriched.ownerKey,
    enriched.projectKey,
    enriched.worktreeKey,
    enriched.lockKind,
    enriched.pathKeys.join("|"),
    enriched.claimGroup ?? "",
  ].join(":");
}

export function applyReleaseEvent(activeClaims, event, overlappingPaths) {
  for (const path of event.paths ?? []) {
    const releaseClaim = {
      writer: event.writer,
      lane: event.lane,
      paths: [path],
      eventId: event.id,
      ...(event.context === undefined ? {} : { context: event.context }),
    };
    for (const [key, claim] of activeClaims) {
      if (releaseMatchesClaim(releaseClaim, claim, overlappingPaths)) {
        activeClaims.delete(key);
      }
    }
  }
}

function releaseMatchesClaim(releaseClaim, activeClaim, overlappingPaths) {
  const release = enrichClaim(releaseClaim);
  const active = enrichClaim(activeClaim);
  if (release.writer !== active.writer) return false;
  if (overlappingPaths(release.paths, active.paths).length === 0) return false;
  const context = releaseClaim.context ?? {};
  if (context.explicitReleaseScope !== true) return true;
  if (hasMeaningfulContext(context, "codexThreadId", "codexSessionId") && release.ownerKey !== active.ownerKey) {
    return false;
  }
  if (hasMeaningfulContext(context, "projectId", "repoRoot", "gitRemote") && !sameReleaseProject(release, active)) {
    return false;
  }
  if (hasMeaningfulContext(context, "worktreeRoot", "repoRoot") && release.worktreeKey !== active.worktreeKey) {
    return false;
  }
  return true;
}

function sameReleaseProject(release, active) {
  if (release.projectKey === active.projectKey) return true;
  if (release.gitRemoteKey !== null && release.gitRemoteKey === active.gitRemoteKey) return true;
  return release.repoRootKey !== null && release.repoRootKey === active.repoRootKey;
}

function hasMeaningfulContext(context, ...keys) {
  return keys.some((key) => {
    const value = context[key];
    return value !== undefined && value !== null && String(value).trim() !== "" && value !== "unknown";
  });
}
