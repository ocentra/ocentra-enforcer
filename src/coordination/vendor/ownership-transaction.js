import { mkdir, rename, rm, rmdir, writeFile } from "node:fs/promises";
import { hostname } from "node:os";
import { join } from "node:path";

import { randomEventId } from "./identity.js";
import { removeStaleOwnershipLock } from "./ownership-lock-state.js";
import { ownershipLockPath, streamsDir } from "./paths.js";

const OWNERSHIP_LOCK_TIMEOUT_MS = 15_000;
const OWNERSHIP_LOCK_RETRY_MS = 10;

/** Serialize ownership decisions and their event append across every writer process. */
export async function withOwnershipTransaction(root, run) {
  await mkdir(streamsDir(root), { recursive: true });
  const path = ownershipLockPath(root);
  const token = randomEventId();
  const ownerName = `${token}.owner.json`;
  const pendingPath = `${path}.${token}.pending`;
  await mkdir(pendingPath);
  await writeFile(join(pendingPath, ownerName), JSON.stringify({
    token,
    pid: process.pid,
    host: hostname(),
    acquiredAt: Date.now(),
  }));
  const deadline = Date.now() + OWNERSHIP_LOCK_TIMEOUT_MS;
  try {
    while (true) {
      try {
        await rename(pendingPath, path);
        break;
      } catch (error) {
        if (!lockIsBusy(error)) throw error;
        if (await removeStaleOwnershipLock(path)) continue;
        if (Date.now() >= deadline) {
          throw new Error(`timed out waiting for ownership transaction lock ${path}`);
        }
        await sleep(OWNERSHIP_LOCK_RETRY_MS);
      }
    }
    return await run();
  } finally {
    await releaseOwnedLock(path, ownerName);
    await removePendingLock(pendingPath, ownerName);
  }
}

async function releaseOwnedLock(path, ownerName) {
  try {
    await rm(join(path, ownerName));
  } catch (error) {
    if (error?.code === "ENOENT") return;
    throw error;
  }
  try {
    await rmdir(path);
  } catch (error) {
    if (!["ENOENT", "ENOTEMPTY", "EPERM"].includes(error?.code)) throw error;
  }
}

async function removePendingLock(path, ownerName) {
  await rm(join(path, ownerName), { force: true });
  try {
    await rmdir(path);
  } catch (error) {
    if (error?.code !== "ENOENT") throw error;
  }
}

function lockIsBusy(error) {
  return ["EEXIST", "ENOTEMPTY", "EPERM"].includes(error?.code);
}

function sleep(ms) {
  // TIMER-JUSTIFICATION: ownership lock retry backoff is bounded and keeps file locks cross-platform.
  return new Promise((resolve) => setTimeout(resolve, ms));
}
