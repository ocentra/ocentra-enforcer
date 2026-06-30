import { mkdir, open, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { streamsDir } from "../paths.js";
import { eventLines, isPrefix } from "./local.js";
export async function syncFromHttpPeer(root, peerUrl, token) {
    await mkdir(streamsDir(root), { recursive: true });
    const base = new URL(peerUrl);
    const streamsResponse = await fetch(new URL("/manifest", base), requestInit(token));
    if (!streamsResponse.ok) {
        throw new Error(`peer streams request failed: ${streamsResponse.status}`);
    }
    const manifest = await streamsResponse.json();
    const streams = normalizeStreams(manifest.streams);
    if (!Array.isArray(streams)) {
        throw new Error("peer manifest response did not contain streams");
    }
    let imported = 0;
    let transferredLines = 0;
    const conflicts = [];
    for (const stream of streams) {
        const streamName = typeof stream === "string" ? stream : stream.stream;
        const localPath = join(streamsDir(root), streamName);
        const remoteEventCount = typeof stream === "string" ? null : stream.eventCount;
        const localLines = await eventLines(localPath);
        if (remoteEventCount !== null && localLines.length > remoteEventCount) {
            const conflictName = await writeConflict(base, token, streamName, root);
            conflicts.push(conflictName);
            continue;
        }
        if (localLines.length > 0) {
            const overlap = await fetchStreamLines(base, token, streamName, localLines.length - 1, 1);
            if (overlap[0] !== localLines.at(-1)) {
                const conflictName = await writeConflict(base, token, streamName, root);
                conflicts.push(conflictName);
                continue;
            }
        }
        const remoteLines = await fetchStreamLines(base, token, streamName, localLines.length);
        transferredLines += remoteLines.length;
        if (!isPrefix(localLines, [...localLines, ...remoteLines])) {
            const conflictName = await writeConflict(base, token, streamName, root);
            conflicts.push(conflictName);
            continue;
        }
        if (remoteLines.length === 0) {
            continue;
        }
        const handle = await open(localPath, "a");
        try {
            for (const line of remoteLines) {
                await handle.appendFile(`${line}\n`);
                imported += 1;
            }
            await handle.sync();
        }
        finally {
            await handle.close();
        }
    }
    return { imported, transferredLines, conflicts };
}
function requestInit(token) {
    return token === undefined || token.length === 0
        ? undefined
        : { headers: { authorization: `Bearer ${token}` } };
}
async function fetchStreamLines(base, token, stream, after = 0, limit = null) {
    const url = new URL(`/streams/${encodeURIComponent(stream)}`, base);
    url.searchParams.set("after", String(after));
    if (limit !== null) url.searchParams.set("limit", String(limit));
    const response = await fetch(url, requestInit(token));
    if (!response.ok) {
        throw new Error(`peer stream request failed for ${stream}: ${response.status}`);
    }
    return (await response.text())
        .split(/\r?\n/)
        .map((line) => line.trim())
        .filter((line) => line.length > 0);
}
async function writeConflict(base, token, stream, root) {
    const response = await fetch(new URL(`/streams/${encodeURIComponent(stream)}`, base), requestInit(token));
    if (!response.ok) {
        throw new Error(`peer conflict stream request failed for ${stream}: ${response.status}`);
    }
    const remoteText = await response.text();
    const conflictName = `${stream}.conflict.${Date.now()}`;
    await writeFile(join(streamsDir(root), conflictName), remoteText.endsWith("\n") ? remoteText : `${remoteText}\n`);
    return conflictName;
}
function normalizeStreams(streams) {
    if (!Array.isArray(streams)) return null;
    if (streams.every((stream) => typeof stream === "string")) return streams;
    if (streams.every((stream) => typeof stream === "object" && stream !== null && typeof stream.stream === "string")) return streams;
    return null;
}
