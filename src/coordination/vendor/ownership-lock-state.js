import { readFile, readdir, rename, rm, rmdir, stat } from "node:fs/promises";
import { randomUUID } from "node:crypto";
import { hostname } from "node:os";
import { dirname, join } from "node:path";

const OWNERSHIP_LOCK_STALE_MS = 60_000;
const TRANSIENT_LOCK_RESULTS = new Map([
  ["ENOENT", true],
  ["ENOTDIR", false],
  ["EPERM", false],
]);

/** Remove an abandoned lock generation without ever deleting its replacement. */
export async function removeStaleOwnershipLock(path) {
  try {
    const lockStat = await stat(path);
    if (Date.now() - lockStat.mtimeMs <= OWNERSHIP_LOCK_STALE_MS) return false;
    const ownerName = (await readdir(path)).find((name) => name.endsWith(".owner.json"));
    if (ownerName === undefined) return removeEmptyStaleLock(path);
    const ownerPath = join(path, ownerName);
    const lockOwner = parseLockOwner(await readFile(ownerPath, "utf8"));
    if (lockOwner?.host === hostname() && processIsAlive(lockOwner.pid)) return false;
    return quarantineStaleOwner(path, ownerName, ownerPath);
  } catch (error) {
    if (TRANSIENT_LOCK_RESULTS.has(error?.code)) return TRANSIENT_LOCK_RESULTS.get(error.code);
    throw error;
  }
}

async function quarantineStaleOwner(path, ownerName, ownerPath) {
  const quarantinePath = join(dirname(path), `${ownerName}.${randomUUID()}.stale`);
  try {
    await rename(ownerPath, quarantinePath);
  } catch {
    return false;
  }
  try {
    await rmdir(path);
    return true;
  } catch {
    return false;
  } finally {
    await rm(quarantinePath, { force: true });
  }
}

async function removeEmptyStaleLock(path) {
  try {
    await rmdir(path);
    return true;
  } catch {
    return false;
  }
}

function parseLockOwner(contents) {
  try {
    return JSON.parse(contents);
  } catch {
    return null;
  }
}

function processIsAlive(pid) {
  try {
    process.kill(pid, 0);
    return true;
  } catch (error) {
    return error?.code !== "ESRCH";
  }
}
