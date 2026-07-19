/** Decide whether an overlapping release may target one active claim identity. */
export function releaseScopeAllows(release, active, context, targetEventIds, activeClaim) {
  if (context.explicitReleaseScope !== true) return true;
  if (targetEventIds?.has(activeClaim.eventId)) return true;
  if (activeClaim.context === undefined) return true;
  if (!sameReleaseOwner(context, activeClaim.context)) return false;
  if (!projectScopeAllows(release, active, context)) return false;
  if (hasMeaningfulContext(context, "worktreeRoot", "repoRoot")) {
    if (release.worktreeKey !== active.worktreeKey) return false;
  }
  return !hasMeaningfulContext(context, "branch") || release.branchKey === active.branchKey;
}

function projectScopeAllows(release, active, context) {
  return (
    !hasMeaningfulContext(context, "projectId", "repoRoot", "gitRemote") ||
    sameReleaseProject(release, active)
  );
}

function sameReleaseOwner(releaseContext, activeContext) {
  const releaseThread = meaningfulOwnerPart(releaseContext.codexThreadId);
  const activeThread = meaningfulOwnerPart(activeContext.codexThreadId);
  const releaseSession = meaningfulOwnerPart(releaseContext.codexSessionId);
  const activeSession = meaningfulOwnerPart(activeContext.codexSessionId);
  const releaseHasOwner = [releaseThread, releaseSession].some(Boolean);
  const activeHasOwner = [activeThread, activeSession].some(Boolean);
  if (releaseHasOwner !== activeHasOwner) return false;
  if (!releaseHasOwner) return true;
  if (bothPresent(releaseThread, activeThread)) return releaseThread === activeThread;
  return releaseSession !== null && releaseSession === activeSession;
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
  return keys.some((key) => meaningfulOwnerPart(context[key]) !== null);
}

function meaningfulOwnerPart(value) {
  const normalized = String(value ?? "").trim();
  return normalized.length === 0 || normalized === "unknown" ? null : normalized;
}

function bothPresent(...values) {
  return values.every((value) => value !== null);
}
