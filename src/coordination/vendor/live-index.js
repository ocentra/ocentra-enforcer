import { readFile } from "node:fs/promises";
import { join } from "node:path";

export const COORDINATION_MATERIALIZER_VERSION = 2;

export async function loadLiveIndex(root) {
    try {
        const parsed = JSON.parse(await readFile(join(root, "db", "coordination-index.json"), "utf8"));
        return parsed?.backend === "json"
            && parsed.materializerVersion === COORDINATION_MATERIALIZER_VERSION
            && Array.isArray(parsed.liveStreams)
            && parsed.liveStreams.every((stream) => typeof stream.digest === "string"
                && (stream.tail === null || (typeof stream.tail?.id === "string" && typeof stream.tail?.hash === "string")))
            && Array.isArray(parsed.state?.seenEventIds)
            && validOrderCursor(parsed.orderCursor ?? parsed.state.orderCursor)
            ? parsed
            : null;
    }
    catch (error) {
        if (isMissingPath(error) || error instanceof SyntaxError) {
            return null;
        }
        throw error;
    }
}

function validOrderCursor(cursor) {
    return cursor !== undefined && typeof cursor?.ts === "string" && typeof cursor?.id === "string";
}

function isMissingPath(error) {
    return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}
