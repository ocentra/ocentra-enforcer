import { appendEvent } from "./stream.js";
import { buildCoordinationContext, claimForLocalSelection } from "./context.js";
import { parseMessageAddress, parseUserText } from "./domain.js";
import { blockersForRequest, normalizeCoordinationPath } from "./lock-policy.js";
import { claimsReleasedByEvent } from "./materialize-claim-identity.js";

/** Select exact active claim paths without widening a partial release. */
export function selectReleaseClaims({
  activeClaims,
  candidates,
  writer,
  lane,
  requestedPaths,
  context,
}) {
  const candidatePaths = requestedPaths.length === 0
    ? unique(candidates.flatMap((claim) => claim.paths ?? []))
    : requestedPaths;
  const selectionEvent = {
    writer,
    lane,
    paths: candidatePaths,
    eventId: "__release__",
    context,
  };
  const selectedClaims = claimsReleasedByEvent(candidates, selectionEvent, claimForLocalSelection);
  const requestedPathKeys = new Set(candidatePaths.map(normalizeCoordinationPath));
  const paths = unique(
    selectedClaims.flatMap((claim) =>
      (claim.paths ?? []).filter((claimPath) =>
        requestedPathKeys.has(normalizeCoordinationPath(claimPath)),
      ),
    ),
  );
  return {
    paths,
    claims: claimsReleasedByEvent(
      activeClaims,
      { ...selectionEvent, paths },
      claimForLocalSelection,
    ),
  };
}

/** Report only selected paths that no longer remain on their original claim event. */
export function releasedSelectionPaths(selection, activeClaims) {
  const selectedPathKeys = new Set(selection.paths.map(normalizeCoordinationPath));
  return unique(
    selection.claims.flatMap((claim) =>
      (claim.paths ?? []).filter((claimPath) => {
        if (!selectedPathKeys.has(normalizeCoordinationPath(claimPath))) return false;
        return !activeClaims.some(
          (active) =>
            active.eventId === claim.eventId &&
            (active.paths ?? []).some(
              (activePath) =>
                normalizeCoordinationPath(activePath) === normalizeCoordinationPath(claimPath),
            ),
        );
      }),
    ),
  );
}

/** Notify only edit intents that are actually unblocked after the release. */
export async function appendReleaseNotifications(root, config, lane, args, event, state, paths) {
  const notificationEvents = [];
  for (const intent of nextEditIntentsForPaths(state.ownership.editIntents ?? [], paths)) {
    const decision = blockersForRequest(
      state.ownership.activeClaims,
      intent,
      intent.context?.operation ?? "edit",
    );
    if (decision.blockers.length > 0) continue;
    notificationEvents.push(
      await appendEvent(root, config, lane, {
        type: "message",
        to: parseMessageAddress(intent.lane),
        body: parseUserText(
          `Released ${paths.join(", ")}. Re-read the file before claiming and editing; queued intent ${intent.eventId}.`,
        ),
        context: buildCoordinationContext({
          ...args,
          releaseEventId: event.id,
          editIntentId: intent.eventId,
          notificationKind: "editIntentReleased",
        }),
      }),
    );
  }
  return notificationEvents;
}

function nextEditIntentsForPaths(editIntents, paths) {
  const normalized = paths.map(normalizeCoordinationPath).filter(Boolean);
  return editIntents.filter((intent) =>
    (intent.paths ?? [])
      .map(normalizeCoordinationPath)
      .some((intentPath) =>
        normalized.some((releasedPath) => pathOverlaps(intentPath, releasedPath)),
      ),
  );
}

function pathOverlaps(left, right) {
  return left === right || left.startsWith(`${right}/`) || right.startsWith(`${left}/`);
}

function unique(values) {
  return [...new Set(values)];
}
