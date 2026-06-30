import { spawnSync } from "node:child_process";
import { stat } from "node:fs/promises";
import { join } from "node:path";
import { parseClaimPath } from "./domain.js";

export const MAX_CLAIM_PATHS = 10;

export function splitClaimPathList(value) {
    return value
        .split(/[,\n]/u)
        .map((item) => item.trim())
        .filter((item) => item.length > 0);
}

export async function normalizeClaimPaths(root, rawPaths) {
    const paths = rawPaths.map(normalizeClaimPathInput).filter((path) => path.length > 0);
    if (paths.length === 0) {
        throw new Error("claim requires at least one exact file path");
    }
    if (paths.length > MAX_CLAIM_PATHS) {
        throw new Error(`claim can cover at most ${MAX_CLAIM_PATHS} files`);
    }
    const seen = new Set();
    const normalized = [];
    for (const claimPath of paths) {
        if (seen.has(claimPath)) {
            throw new Error(`duplicate claim path: ${claimPath}`);
        }
        if (!isExactClaimPathCandidate(claimPath)) {
            throw new Error(`claim paths must be exact files, not folders or globs: ${claimPath}`);
        }
        if (await isExistingDirectory(root, claimPath)) {
            throw new Error(`claim paths must be exact files, not folders: ${claimPath}`);
        }
        if (await hasKnownDescendants(root, claimPath)) {
            throw new Error(`claim paths must be exact files, not folder prefixes: ${claimPath}`);
        }
        seen.add(claimPath);
        normalized.push(parseClaimPath(claimPath));
    }
    return normalized;
}

export async function isFolderLikeClaimPath(root, claimPath) {
    const normalized = normalizeClaimPathInput(claimPath);
    if (!isExactClaimPathCandidate(normalized)) {
        return true;
    }
    if (await isExistingDirectory(root, normalized)) {
        return true;
    }
    return hasKnownDescendants(root, normalized);
}

export function normalizeClaimPathInput(raw) {
    return String(raw)
        .trim()
        .replace(/\\/gu, "/")
        .replace(/\/+/gu, "/")
        .replace(/^\.\//u, "");
}

export function isExactClaimPathCandidate(claimPath) {
    if (claimPath.length === 0) return false;
    if (claimPath.includes("*") || claimPath.includes("?")) return false;
    if (claimPath.endsWith("/")) return false;
    if (claimPath.startsWith("/") || /^[A-Za-z]:\//u.test(claimPath)) return false;
    if (claimPath.split("/").includes("..")) return false;
    return true;
}

async function isExistingDirectory(root, claimPath) {
    try {
        return (await stat(join(root, claimPath))).isDirectory();
    } catch {
        return false;
    }
}

async function hasKnownDescendants(root, claimPath) {
    const result = spawnSync("git", ["ls-files", "-co", "--exclude-standard", "--", `${claimPath}/`], {
        cwd: root,
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        windowsHide: true,
    });
    if ((result.status ?? 1) !== 0) {
        return false;
    }
    return result.stdout.trim().length > 0;
}
