import { spawnSync } from "node:child_process";
import { existsSync, realpathSync } from "node:fs";
import { arch, hostname, platform, release, userInfo } from "node:os";
import { basename, resolve } from "node:path";

export function buildCoordinationContext(input = {}) {
    const cwd = canonicalExistingCoordinationPath(resolve(input.cwd ?? process.cwd()));
    const repoRoot = canonicalExistingCoordinationPath(resolve(input.repoRoot ?? input.root ?? gitValue(cwd, ["rev-parse", "--show-toplevel"]) ?? cwd));
    const worktreeRoot = canonicalExistingCoordinationPath(resolve(input.worktreeRoot ?? gitValue(repoRoot, ["rev-parse", "--show-toplevel"]) ?? repoRoot));
    const gitRemote = input.gitRemote ?? gitValue(worktreeRoot, ["config", "--get", "remote.origin.url"]) ?? null;
    const explicitProjectId = Object.hasOwn(input, "explicitProjectId")
        ? input.explicitProjectId
        : input.projectId ?? process.env.OCENTRA_PROJECT_ID ?? null;
    const context = {
        machine: input.machine ?? hostname(),
        user: input.user ?? currentUser(),
        os: input.os ?? `${platform()} ${release()} ${arch()}`,
        hub: input.hub ?? process.env.OCENTRA_COORDINATION_HUB ?? process.env.OCENTRA_ENFORCER_HUB ?? null,
        projectId: input.projectId ?? process.env.OCENTRA_PROJECT_ID ?? deriveProjectId(gitRemote, repoRoot),
        explicitProjectId,
        repoRoot,
        worktreeRoot,
        gitRemote,
        branch: input.branch ?? gitValue(worktreeRoot, ["branch", "--show-current"]) ?? null,
        commit: input.commit ?? gitValue(worktreeRoot, ["rev-parse", "--short", "HEAD"]) ?? null,
        cwd,
        codexThreadId: input.codexThreadId ?? process.env.CODEX_THREAD_ID ?? process.env.CODEX_THREAD ?? "unknown",
        codexSessionId: input.codexSessionId ?? process.env.CODEX_SESSION_ID ?? process.env.CODEX_SESSION ?? "unknown",
        pid: input.pid ?? process.pid,
        enforcerVersion: input.enforcerVersion ?? process.env.npm_package_version ?? "0.1.0",
        peerUrl: input.peerUrl ?? null,
        syncStatus: input.syncStatus ?? "unknown",
        operation: input.operation,
        lockKind: input.lockKind,
        onConflict: input.onConflict,
        claimGroup: input.claimGroup,
        intentFor: input.intentFor,
        blockingOwners: input.blockingOwners,
        blockerCount: input.blockerCount,
        releaseEventId: input.releaseEventId,
        releaseClaimEventIds: input.releaseClaimEventIds,
        explicitReleaseScope: input.explicitReleaseScope,
        editIntentId: input.editIntentId,
        notificationKind: input.notificationKind,
    };
    return Object.fromEntries(Object.entries(context).filter((entry) => entry[1] !== undefined));
}

/** Resolve local aliases while preserving serialized paths that do not exist on this machine. */
export function canonicalExistingCoordinationPath(value) {
    const candidate = String(value ?? "").trim();
    if (candidate.length === 0 || !existsSync(candidate)) return candidate;
    try {
        return realpathSync.native(candidate);
    }
    catch {
        return candidate;
    }
}

/** Resolve aliases only while a local command selects serialized claims to target. */
export function claimForLocalSelection(claim) {
    if (claim.context === undefined) return claim;
    const context = claim.context;
    return {
        ...claim,
        context: {
            ...context,
            ...(context.repoRoot === undefined
                ? {}
                : { repoRoot: canonicalExistingCoordinationPath(context.repoRoot) }),
            ...(context.worktreeRoot === undefined
                ? {}
                : { worktreeRoot: canonicalExistingCoordinationPath(context.worktreeRoot) }),
        },
    };
}

function gitValue(cwd, args) {
    if (!existsSync(cwd)) return null;
    const result = spawnSync("git", args, {
        cwd,
        encoding: "utf8",
        windowsHide: true,
    });
    if (result.status !== 0) return null;
    const value = result.stdout.trim();
    return value.length === 0 ? null : value;
}

function currentUser() {
    try {
        return userInfo().username;
    }
    catch {
        return process.env.USERNAME ?? process.env.USER ?? "unknown";
    }
}

function deriveProjectId(gitRemote, repoRoot) {
    if (typeof gitRemote === "string" && gitRemote.length > 0) {
        const normalized = gitRemote.replace(/\.git$/u, "").replace(/\\/gu, "/");
        const parts = normalized.split(/[/:]/u).filter(Boolean);
        const owner = parts.at(-2);
        const repo = parts.at(-1);
        if (owner !== undefined && repo !== undefined) {
            return sanitizeProjectId(`${owner}-${repo}`);
        }
    }
    return sanitizeProjectId(basename(repoRoot));
}

function sanitizeProjectId(value) {
    return String(value)
        .replaceAll(/[^A-Za-z0-9._-]+/gu, "-")
        .replaceAll(/^-|-$/gu, "")
        .slice(0, 120) || "unknown-project";
}
