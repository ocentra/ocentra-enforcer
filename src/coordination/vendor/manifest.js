import { createHash } from "node:crypto";
import { readFile, stat } from "node:fs/promises";
import { relative } from "node:path";
import { streamFiles } from "./sync/local.js";
import { streamSegments } from "./stream.js";

export async function buildStreamManifest(root) {
    const streams = [];
    for (const stream of await streamFiles(root)) {
        const segments = [];
        let eventCount = 0;
        let byteLength = 0;
        let firstSeq = null;
        let lastSeq = null;
        let lastEventId = null;
        let tailHash = null;
        for (const segment of await streamSegments(root, stream)) {
            const summary = await summarizeSegment(root, segment);
            if (summary === null) continue;
            segments.push(summary);
            eventCount += summary.eventCount;
            byteLength += summary.byteLength;
            firstSeq ??= summary.firstSeq;
            lastSeq = summary.lastSeq;
            lastEventId = summary.lastEventId;
            tailHash = summary.tailHash;
        }
        streams.push({
            stream,
            writer: stream.slice(0, -".ndjson".length),
            eventCount,
            byteLength,
            firstSeq,
            lastSeq,
            lastEventId,
            tailHash,
            segments,
        });
    }
    return {
        ok: true,
        root,
        generatedAt: new Date().toISOString(),
        streams,
    };
}

export async function readStreamTextRange(path, after = 0, limit = null) {
    const lines = await readLines(path);
    const start = Math.max(0, Number(after) || 0);
    const end = limit === null ? undefined : start + Math.max(0, Number(limit) || 0);
    const selected = lines.slice(start, end);
    return selected.length === 0 ? "" : `${selected.join("\n")}\n`;
}

async function summarizeSegment(root, segment) {
    const lines = await readLines(segment.path);
    if (lines.length === 0) return null;
    const stats = await stat(segment.path);
    const first = parseLine(lines[0]);
    const last = parseLine(lines.at(-1));
    return {
        path: relative(root, segment.path).replace(/\\/gu, "/"),
        archived: segment.archived,
        byteLength: stats.size,
        eventCount: lines.length,
        sha256: sha256(lines.join("\n")),
        firstSeq: first?.seq ?? null,
        lastSeq: last?.seq ?? null,
        firstEventId: first?.id ?? null,
        lastEventId: last?.id ?? null,
        tailHash: last?.hash ?? null,
    };
}

async function readLines(path) {
    try {
        return (await readFile(path, "utf8"))
            .split(/\r?\n/u)
            .map((line) => line.trim())
            .filter(Boolean);
    }
    catch (error) {
        if (typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT") {
            return [];
        }
        throw error;
    }
}

function parseLine(line) {
    try {
        return JSON.parse(line);
    }
    catch {
        return null;
    }
}

function sha256(value) {
    return createHash("sha256").update(value).digest("hex");
}
