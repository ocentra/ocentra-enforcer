import { enrichClaim, normalizeCoordinationPath } from "./lock-policy.js";
import { releaseScopeAllows } from "./release-scope.js";

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
  const matchRelease = releaseEventMatch(event, overlappingPaths);
  const remainders = [];
  for (const [key, claim] of activeClaims) {
    const match = matchRelease(claim);
    if (!match.matches) continue;
    activeClaims.delete(key);
    const releasedPathKeys = new Set(match.paths.map(normalizeCoordinationPath));
    const remainingPaths = claim.paths.filter(
      (path) => !releasedPathKeys.has(normalizeCoordinationPath(path)),
    );
    if (remainingPaths.length > 0) {
      const remainder = { ...claim, paths: remainingPaths };
      remainders.push(remainder);
    }
  }
  for (const remainder of remainders) {
    activeClaims.set(claimIdentityKey(remainder), remainder);
  }
}

/** Return the active claims that replaying one release event would remove. */
export function claimsReleasedByEvent(activeClaims, event, claimForMatch = (claim) => claim) {
  const matchesRelease = releaseEventMatcher(event);
  return activeClaims.filter((claim) => matchesRelease(claimForMatch(claim)));
}

/** Record selected claim identities while retaining historical scoped release metadata. */
export function releaseContextForClaims(context, claims) {
  return {
    ...(context ?? {}),
    releaseClaimEventIds: [
      ...new Set(claims.map((claim) => claim.eventId).filter((value) => typeof value === "string")),
    ],
    explicitReleaseScope: true,
  };
}

function releaseEventMatcher(event, overlappingPaths = exactOverlappingPaths) {
  const matchRelease = releaseEventMatch(event, overlappingPaths);
  return (activeClaim) => matchRelease(activeClaim).matches;
}

function releaseEventMatch(event, overlappingPaths = exactOverlappingPaths) {
  const release = enrichClaim({
    writer: event.writer,
    lane: event.lane,
    paths: event.paths ?? [],
    eventId: event.id,
    ...(event.context === undefined ? {} : { context: event.context }),
  });
  const context = event.context ?? {};
  const targetEventIds = recordedReleaseClaimEventIds(context);
  return (activeClaim) => {
    const active = enrichClaim(activeClaim);
    const paths = overlappingPaths(active.paths, release.paths);
    const noMatch = { matches: false, paths: [] };
    const match = { matches: true, paths };
    if (release.writer !== active.writer || paths.length === 0) return noMatch;
    return releaseScopeAllows(release, active, context, targetEventIds, activeClaim)
      ? match
      : noMatch;
  };
}

function recordedReleaseClaimEventIds(context) {
  if (!Array.isArray(context.releaseClaimEventIds)) return null;
  return new Set(context.releaseClaimEventIds.filter((value) => typeof value === "string"));
}

function exactOverlappingPaths(left, right) {
  const normalizedRight = new Set(right.map(normalizeCoordinationPath));
  return left.map(normalizeCoordinationPath).filter((leftPath) => normalizedRight.has(leftPath));
}
