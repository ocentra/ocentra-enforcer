import { parseLaneId } from "./domain.js";
import { inspectLedger } from "./doctor.js";
import { materialize } from "./materialize.js";
import { isFolderLikeClaimPath } from "./claim-policy.js";
import { buildCoordinationContext } from "./context.js";
import {
    blockersForRequest,
    buildRequestClaim,
    claimMatchesOperation,
    conflictTouchesPaths,
    normalizeCoordinationPath,
    normalizeLockKind,
    normalizeOperation,
    pathOverlaps,
} from "./lock-policy.js";
export async function guardLedger(root, input) {
    const lane = parseLaneId(input.lane);
    const operation = normalizeOperation(input.operation, "commit");
    const inspection = await inspectLedger(root);
    const findings = [];
    const globalWarnings = [];
    let globalWarningCount = 0;
    const globalWarningLimit = Number.isFinite(input.limit)
        ? Math.max(0, Number(input.limit))
        : 20;
    const noteGlobalWarning = (message) => {
        globalWarningCount += 1;
        if (globalWarnings.length < globalWarningLimit) {
            globalWarnings.push(message);
        }
    };
    const changedPaths = (input.changedPaths ?? [])
        .map(normalizeCoordinationPath)
        .filter((path) => path.length > 0);
    const focused = changedPaths.length > 0 && input.focused !== false;
    let state;
    try {
        state = await materialize(root);
    }
    catch (error) {
        findings.push(`ledger materialization failed: ${error instanceof Error ? error.message : String(error)}`);
        return {
            ok: false,
            lane,
            findings,
            globalWarnings,
            globalWarningCount,
            globalWarningsTruncated: false,
            focused,
            changedPaths,
            operation,
            mustRepairLedger: true,
            diagnostics: inspection.diagnostics.slice(0, globalWarningLimit),
            diagnosticCount: inspection.diagnostics.length,
            diagnosticsTruncated: inspection.diagnostics.length > globalWarningLimit,
        };
    }
    const laneView = state.lanes.get(lane);
    const activeSession = state.sessions.get(lane);
    if (input.sessionId !== undefined && activeSession !== undefined && activeSession.sessionId !== input.sessionId) {
        findings.push(`lane ${lane} is owned by active session ${activeSession.sessionId}`);
    }
    const unread = laneView?.inbox.filter((item) => item.ackedBy.length === 0) ?? [];
    if (operation !== "inspect" && lane !== "primary" && unread.length > 0) {
        findings.push(`lane ${lane} has ${unread.length} unread ledger message(s)`);
    }
    const requestContext = buildCoordinationContext({
        ...input,
        repoRoot: input.repoRoot ?? input.root,
        cwd: input.cwd ?? input.root,
    });
    const requestLockKind = operation === "push" ? "branchLease" : normalizeLockKind(input.lockKind, "writeLock");
    const claimsForDecision = state.ownership.activeClaims.filter((claim) => claim.lane !== lane);
    const decision = changedPaths.length > 0
        ? blockersForRequest(claimsForDecision, buildRequestClaim({
            writer: `request.${lane}`,
            lane,
            paths: changedPaths,
            context: {
                ...requestContext,
                operation,
                lockKind: requestLockKind,
                ...(input.claimGroup ? { claimGroup: input.claimGroup } : {}),
            },
        }), operation)
        : null;
    const hardConflicts = state.ownership.hardConflicts ?? state.ownership.conflicts ?? [];
    for (const conflict of hardConflicts) {
        const message = conflictMessage(conflict);
        if (operation === "inspect") {
            noteGlobalWarning(message);
        }
        else if (decision === null && (!focused || conflictTouchesPaths(conflict, changedPaths))) {
            findings.push(message);
        }
        else if (decision === null) {
            noteGlobalWarning(message);
        }
        else if (!decision.blockers.includes(conflict) && !focused) {
            findings.push(message);
        }
        else if (!decision.blockers.includes(conflict) && !conflictTouchesPaths(conflict, changedPaths)) {
            noteGlobalWarning(message);
        }
    }
    if (decision !== null) {
        if (operation === "inspect") {
            for (const warning of decision.hardConflicts) {
                noteGlobalWarning(conflictMessage(warning));
            }
        }
        for (const blocker of decision.blockers) {
            findings.push(conflictMessage(blocker));
        }
        for (const risk of decision.mergeRisks) {
            const message = conflictMessage(risk);
            if (operation === "pr_ready" && input.allowMergeRisks !== true) {
                findings.push(message);
            }
            else {
                noteGlobalWarning(message);
            }
        }
    }
    const requiresClaim = operation === "commit";
    if (requiresClaim && changedPaths.length > 0 && (lane !== "primary" || input.allowPrimaryWithoutClaims !== true)) {
        const laneClaims = state.ownership.activeClaims.filter((claim) => claim.lane === lane);
        if (laneClaims.length === 0) {
            const unownedPaths = [];
            for (const path of changedPaths) {
                const ownerClaims = ownerClaimsForPath(state.ownership.activeClaims, path, lane);
                if (ownerClaims.length > 0) {
                    findings.push(`changed path ${path} is claimed by ${ownerClaims.map(claimOwnerLabel).join(", ")}; lane ${lane} cannot write it`);
                }
                else {
                    unownedPaths.push(path);
                }
            }
            if (unownedPaths.length > 0) {
                findings.push(`lane ${lane} has changed files but no active ledger claim: ${unownedPaths.join(", ")}`);
            }
        }
        else {
            for (const path of changedPaths) {
                if (!laneClaims.some((claim) => claimMatchesOperation(claim, path, operation, { ...requestContext, lane }))) {
                    const ownerClaims = ownerClaimsForPath(state.ownership.activeClaims, path, lane);
                    if (ownerClaims.length > 0) {
                        findings.push(`changed path ${path} is claimed by ${ownerClaims.map(claimOwnerLabel).join(", ")}; lane ${lane} cannot write it`);
                    }
                    else {
                        findings.push(`changed path ${path} is outside active ledger claims for lane ${lane}`);
                    }
                }
            }
        }
    }
    for (const claim of state.ownership.activeClaims.filter((item) => item.lane === lane)) {
        for (const path of claim.paths) {
            const normalizedClaimPath = normalizeCoordinationPath(path);
            if ((!focused || changedPaths.some((changedPath) => pathOverlaps(changedPath, normalizedClaimPath))) && await isFolderLikeClaimPath(root, String(path))) {
                findings.push(`lane ${lane} has non-exact claim path ${path}; claims must be exact files`);
            }
        }
    }
    for (const diagnostic of inspection.diagnostics) {
        if (diagnostic.level === "error") {
            const message = `${diagnostic.stream}: ${diagnostic.message}`;
            if (!focused || diagnosticBlocksFocusedGuard(diagnostic, lane)) {
                findings.push(message);
            }
            else {
                noteGlobalWarning(message);
            }
        }
    }
    return {
        ok: findings.length === 0,
        lane,
        findings,
        globalWarnings,
        globalWarningCount,
        globalWarningsTruncated: globalWarningCount > globalWarnings.length,
        focused,
        changedPaths,
        operation,
        blockers: decision?.blockers ?? [],
        mergeRisks: decision?.mergeRisks ?? [],
        hardConflictCount: hardConflicts.length,
        editIntentCount: state.ownership.editIntents?.length ?? 0,
        diagnostics: inspection.diagnostics.slice(0, globalWarningLimit),
        diagnosticCount: inspection.diagnostics.length,
        diagnosticsTruncated: inspection.diagnostics.length > globalWarningLimit,
    };
}
function claimMatchesPath(claim, path) {
    return claim.paths.some((claimPath) => pathMatchesClaim(path, normalizeCoordinationPath(claimPath)));
}
function ownerClaimsForPath(claims, path, currentLane) {
    return claims.filter((claim) => claim.lane !== currentLane && claimMatchesPath(claim, path));
}
function claimOwnerLabel(claim) {
    return `${claim.lane} (${claim.writer}, ${claim.lockKind ?? "writeLock"})`;
}
function pathMatchesClaim(path, claimPath) {
    return normalizeCoordinationPath(path) === claimPath;
}
function conflictMessage(conflict) {
    const type = conflict.type ?? "ownership-conflict";
    const owners = (conflict.owners ?? []).length > 0
        ? conflict.owners.map((owner) => `${owner.lane} (${owner.writer}, ${owner.lockKind})`).join(", ")
        : (conflict.lanes ?? []).join(", ");
    return `${type}: ${(conflict.paths ?? []).join(", ")} owned by ${owners}`;
}
function diagnosticBlocksFocusedGuard(diagnostic, lane) {
    const message = String(diagnostic.message ?? "");
    if (/hash-invalid|malformed event|first event|previous event pointer/iu.test(message)) {
        return true;
    }
    return streamLane(diagnostic.stream) === lane;
}
function streamLane(stream) {
    const name = String(stream ?? "").split(/[\\/]/u).at(0) ?? "";
    const withoutExtension = name.replace(/\.ndjson$/u, "");
    const lastDot = withoutExtension.lastIndexOf(".");
    return lastDot >= 0 ? withoutExtension.slice(lastDot + 1) : "";
}
