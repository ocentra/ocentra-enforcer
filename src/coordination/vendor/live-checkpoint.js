import { open } from "node:fs/promises";
import { assertEventHash, parseHubEvent } from "./events.js";
import { prefixDigest } from "./live-prefix.js";

const CHECKPOINT_TAIL_BYTES = 64 * 1024;

/** Capture a stream's byte offset, prefix digest, and bounded tail identity. */
export async function checkpointOffset(path, stream) {
    const handle = await open(path, "r");
    try {
        const byteLength = (await handle.stat()).size;
        return {
            stream,
            byteLength,
            digest: await prefixDigest(handle, byteLength),
            tail: await readTail(handle, byteLength),
        };
    }
    finally {
        await handle.close();
    }
}

/** Read the final event identity at a previously captured stream offset. */
export async function checkpointTail(path, offset) {
    const handle = await open(path, "r");
    try {
        return await readTail(handle, offset);
    }
    finally {
        await handle.close();
    }
}

/** Recompute the prefix digest at a previously captured stream offset. */
export async function checkpointDigest(path, offset) {
    const handle = await open(path, "r");
    try {
        return await prefixDigest(handle, offset);
    }
    finally {
        await handle.close();
    }
}

/** Return whether two checkpoint tails identify the same final event. */
export function sameCheckpointTail(left, right) {
    return left?.id === right?.id && left?.hash === right?.hash;
}

/** Return whether an I/O error reports a missing stream path. */
export function isMissingPath(error) {
    return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}

async function readTail(handle, end) {
    if (end === 0) {
        return null;
    }
    const start = Math.max(0, end - CHECKPOINT_TAIL_BYTES);
    const buffer = Buffer.alloc(end - start);
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, start);
    if (bytesRead !== buffer.length) {
        const error = new Error("stream changed while reading checkpoint tail");
        error.code = "ENOENT";
        throw error;
    }
    const text = buffer.toString("utf8");
    const line = text.split(/\r?\n/u).findLast((entry) => entry.trim().length > 0);
    if (line === undefined || (start > 0 && !text.includes("\n"))) {
        throw new Error("unable to read bounded checkpoint tail");
    }
    const event = parseHubEvent(JSON.parse(line));
    assertEventHash(event);
    return { id: event.id, hash: event.hash };
}
