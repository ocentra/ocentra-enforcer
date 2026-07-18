import { open, readFile } from "node:fs/promises";
import { join } from "node:path";
import { assertEventHash, parseHubEvent } from "./events.js";
import { streamsDir } from "./paths.js";
import { listStreamFiles } from "./stream.js";
import { checkpointDigest, checkpointOffset, checkpointTail, isMissingPath, sameCheckpointTail } from "./live-checkpoint.js";
import { samePrefixDigest } from "./live-prefix.js";

export async function liveStreamOffsets(root) {
    const offsets = [];
    try {
        for (const stream of await listStreamFiles(root)) {
            offsets.push(await checkpointOffset(join(streamsDir(root), stream), stream));
        }
        return offsets;
    }
    catch (error) {
        if (isMissingPath(error)) {
            return null;
        }
        throw error;
    }
}

export async function readLiveDeltas(root, offsets) {
    const known = new Map(offsets.map((entry) => [entry.stream, entry]));
    try {
        const streams = await listStreamFiles(root);
        if ([...known.keys()].some((stream) => !streams.includes(stream))) {
            return null;
        }
        const events = [];
        for (const stream of streams) {
            const path = join(streamsDir(root), stream);
            const checkpoint = known.get(stream);
            const offset = checkpoint?.byteLength ?? 0;
            const handle = await open(path, "r");
            let size;
            try {
                size = (await handle.stat()).size;
            }
            finally {
                await handle.close();
            }
            if (size < offset || (checkpoint !== undefined && (!samePrefixDigest(checkpoint.digest, await checkpointDigest(path, offset))
                || !sameCheckpointTail(checkpoint.tail, await checkpointTail(path, offset))))) {
                return null;
            }
            if (size > offset) {
                events.push(...parseDeltaEvents(await readFile(path, "utf8"), offset));
            }
        }
        events.sort(compareEvents);
        return { events };
    }
    catch (error) {
        if (isMissingPath(error)) {
            return null;
        }
        throw error;
    }
}

function parseDeltaEvents(text, offset) {
    const events = [];
    for (const line of Buffer.from(text).subarray(offset).toString("utf8").split(/\r?\n/u)) {
        if (line.trim().length === 0) {
            continue;
        }
        const event = parseHubEvent(JSON.parse(line));
        assertEventHash(event);
        events.push(event);
    }
    return events;
}

function compareEvents(left, right) {
    const ts = left.ts.localeCompare(right.ts);
    return ts === 0 ? left.id.localeCompare(right.id) : ts;
}
