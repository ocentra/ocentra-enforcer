import { mkdir, open, readdir, readFile, rm } from "node:fs/promises";
import { basename, join } from "node:path";
import { buildCoordinationContext } from "./context.js";
import { nowIso, parseEventType, writerId, } from "./domain.js";
import { assertCoordinationHashCompatibility, assertEventHash, completeEvent, parseHubEvent } from "./events.js";
import { randomEventId } from "./identity.js";
import { archivedStreamDir, archiveStreamsDir, lockPath, streamPath, streamsDir } from "./paths.js";
export async function appendEvent(root, config, lane, event) {
    await mkdir(streamsDir(root), { recursive: true });
    return withStreamLock(root, config, lane, async () => {
        const path = streamPath(root, config.nodeId, lane);
        const events = await readCanonicalStream(root, basename(path));
        const input = eventInput(config, lane, event);
        assertCoordinationHashCompatibility();
        const completed = completeEvent(input, events.at(-1), randomEventId());
        const handle = await open(path, "a");
        try {
            await handle.appendFile(`${JSON.stringify(completed)}\n`);
            await handle.sync();
        }
        finally {
            await handle.close();
        }
        return completed;
    });
}
export async function listStreamFiles(root) {
    try {
        const names = await readdir(streamsDir(root));
        return names.filter((name) => name.endsWith(".ndjson") && !name.includes(".conflict.")).sort();
    }
    catch (error) {
        if (isMissingPath(error)) {
            return [];
        }
        throw error;
    }
}
export async function readAllStreams(root) {
    const seen = new Set();
    const events = [];
    const warnings = [];
    let duplicateCount = 0;
    for (const fileName of await listStreamFiles(root)) {
        for (const segment of await streamSegments(root, fileName)) {
            for (const parsed of await readStreamLenient(segment.path)) {
                if (parsed.kind === "warning") {
                    warnings.push(`${basename(segment.path)}:${parsed.line}: ${parsed.warning}`);
                    continue;
                }
                if (seen.has(parsed.event.id)) {
                    duplicateCount += 1;
                    continue;
                }
                seen.add(parsed.event.id);
                events.push(parsed.event);
            }
        }
    }
    events.sort((left, right) => {
        const ts = left.ts.localeCompare(right.ts);
        return ts === 0 ? left.id.localeCompare(right.id) : ts;
    });
    return { events, duplicateCount, warnings };
}
export async function listCanonicalStreamNames(root) {
    const names = new Set(await listStreamFiles(root));
    try {
        for (const streamName of await readdir(archiveStreamsDir(root))) {
            if (streamName.endsWith(".ndjson") && !streamName.includes(".conflict.")) {
                names.add(streamName);
            }
        }
    }
    catch (error) {
        if (!isMissingPath(error)) {
            throw error;
        }
    }
    return [...names].sort();
}
export async function streamSegments(root, streamName) {
    const segments = [];
    try {
        const archiveNames = (await readdir(archivedStreamDir(root, streamName)))
            .filter((name) => name.endsWith(".ndjson"))
            .sort();
        for (const archiveName of archiveNames) {
            segments.push({
                streamName,
                path: join(archivedStreamDir(root, streamName), archiveName),
                archived: true,
            });
        }
    }
    catch (error) {
        if (!isMissingPath(error)) {
            throw error;
        }
    }
    segments.push({
        streamName,
        path: join(streamsDir(root), streamName),
        archived: false,
    });
    return segments;
}
export async function readStream(path) {
    const parsed = await readStreamLenient(path);
    const events = [];
    for (const item of parsed) {
        if (item.kind === "event") {
            assertEventHash(item.event);
            events.push(item.event);
        }
        else {
            throw new Error(`${path}:${item.line}: ${item.warning}`);
        }
    }
    return events;
}
async function readCanonicalStream(root, streamName) {
    const events = [];
    for (const segment of await streamSegments(root, streamName)) {
        events.push(...await readStream(segment.path));
    }
    return events;
}
async function withStreamLock(root, config, lane, run) {
    const path = lockPath(root, config.nodeId, lane);
    const deadline = Date.now() + 5000;
    await mkdir(streamsDir(root), { recursive: true });
    while (true) {
        let handle;
        try {
            handle = await open(path, "wx");
            await handle.writeFile(String(process.pid));
            break;
        }
        catch (error) {
            if (!isAlreadyExists(error) || Date.now() >= deadline) {
                throw error;
            }
            await sleep(25);
        }
        finally {
            await handle?.close();
        }
    }
    try {
        return await run();
    }
    finally {
        await rm(path, { force: true });
    }
}
function eventInput(config, lane, command) {
    const base = {
        schema: 1,
        hub: config.hub,
        nodeId: config.nodeId,
        nodeName: config.nodeName,
        lane,
        writer: writerId(config.nodeId, lane),
        type: parseEventType(command.type),
        ts: nowIso(),
        context: buildCoordinationContext({
            ...(command.context ?? {}),
            hub: config.hub,
        }),
    };
    switch (command.type) {
        case "lane.register":
            return base;
        case "message":
            return { ...base, to: command.to, body: command.body };
        case "ack":
            return { ...base, messageId: command.messageId };
        case "claim":
            return { ...base, paths: command.paths, reason: command.reason };
        case "editIntent":
            return { ...base, paths: command.paths, reason: command.reason };
        case "release":
            return { ...base, paths: command.paths, reason: command.reason };
        case "claim.resolve":
            return {
                ...base,
                paths: command.paths,
                ...(command.owner === undefined ? {} : { owner: command.owner }),
            };
        case "status":
            return { ...base, state: command.state, summary: command.summary };
        case "heartbeat":
            return {
                ...base,
                state: command.state,
                summary: command.summary,
                ttlSeconds: command.ttlSeconds,
            };
        case "session.claim":
            return {
                ...base,
                sessionId: command.sessionId,
                ttlSeconds: command.ttlSeconds,
                ...(command.summary === undefined ? {} : { summary: command.summary }),
            };
        case "session.release":
            return {
                ...base,
                sessionId: command.sessionId,
                ...(command.summary === undefined ? {} : { summary: command.summary }),
            };
        case "worker.update":
            return {
                ...base,
                workerState: command.workerState,
                summary: command.summary,
                ...(command.taskId === undefined ? {} : { taskId: command.taskId }),
            };
        case "task.update":
            return {
                ...base,
                taskId: command.taskId,
                taskState: command.taskState,
                summary: command.summary,
                ...(command.title === undefined ? {} : { title: command.title }),
                ...(command.prUrl === undefined ? {} : { prUrl: command.prUrl }),
            };
        case "report":
            return {
                ...base,
                summary: command.summary,
                ...(command.taskId === undefined ? {} : { taskId: command.taskId }),
            };
        case "handoff":
            return { ...base, to: command.to, body: command.body };
        case "note":
            return { ...base, body: command.body };
    }
}
async function readStreamLenient(path) {
    let raw;
    try {
        raw = await readFile(path, "utf8");
    }
    catch (error) {
        if (isMissingPath(error)) {
            return [];
        }
        throw error;
    }
    const lines = raw.split(/\r?\n/);
    const parsed = [];
    for (const [index, line] of lines.entries()) {
        if (line.trim().length === 0) {
            continue;
        }
        try {
            parsed.push({ kind: "event", event: parseHubEvent(JSON.parse(line)) });
        }
        catch (error) {
            const isFinalLine = index === lines.length - 1 || (index === lines.length - 2 && lines.at(-1) === "");
            parsed.push({
                kind: "warning",
                line: index + 1,
                warning: isFinalLine ? "ignored malformed final line" : `malformed line: ${String(error)}`,
            });
        }
    }
    return parsed;
}
function isMissingPath(error) {
    return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}
function isAlreadyExists(error) {
    return typeof error === "object" && error !== null && "code" in error && error.code === "EEXIST";
}
function sleep(ms) {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
