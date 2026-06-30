import { copyFile, mkdir, open, readdir, readFile, rename, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { inspectLedger } from "./doctor.js";
import { hashForEvent, hashForEventWithExtensions } from "./events.js";
import { rebuildCoordinationIndex } from "./read-index.js";
import { streamsDir } from "./paths.js";

export async function repairLegacyHashCompatibility(root, options = {}) {
    const dryRun = options.dryRun !== false;
    const streams = await streamFiles(root);
    const repairedStreams = [];
    const skippedStreams = [];
    let repairedEvents = 0;
    let rehashedEvents = 0;
    for (const streamName of streams) {
        const streamPath = join(streamsDir(root), streamName);
        const result = dryRun
            ? await repairStream(streamPath, { dryRun })
            : await withStreamFileLock(streamPath, () => repairStream(streamPath, { dryRun }));
        if (result.errors.length > 0) {
            skippedStreams.push({
                stream: streamName,
                errors: result.errors,
            });
            continue;
        }
        if (!result.changed)
            continue;
        repairedEvents += result.repairedEvents;
        rehashedEvents += result.rehashedEvents;
        repairedStreams.push({
            stream: streamName,
            repairedEvents: result.repairedEvents,
            rehashedEvents: result.rehashedEvents,
            backupPath: result.backupPath,
        });
    }
    const inspection = dryRun ? undefined : await inspectLedger(root);
    const index = dryRun ? undefined : await rebuildCoordinationIndex(root, { limit: options.limit });
    return {
        ok: skippedStreams.length === 0 && (inspection?.ok ?? true),
        root,
        dryRun,
        repairedEvents,
        rehashedEvents,
        repairedStreams,
        skippedStreams,
        ...(inspection === undefined ? {} : { inspection }),
        ...(index === undefined ? {} : { index }),
        nextStep: dryRun
            ? "Re-run with dryRun=false or CLI --write after reviewing repairedStreams."
            : "Run coordination doctor/health, then legacy ledger:doctor if this was a transition repair.",
    };
}

export async function repairSequenceBreaks(root, options = {}) {
    const dryRun = options.dryRun !== false;
    const streams = await streamFiles(root);
    const repairedStreams = [];
    const skippedStreams = [];
    let repairedEvents = 0;
    let sequenceRepairs = 0;
    let pointerRepairs = 0;
    let rehashedEvents = 0;
    for (const streamName of streams) {
        const streamPath = join(streamsDir(root), streamName);
        const result = dryRun
            ? await repairSequenceStream(streamPath, { dryRun })
            : await withStreamFileLock(streamPath, () => repairSequenceStream(streamPath, { dryRun }));
        if (result.errors.length > 0) {
            skippedStreams.push({
                stream: streamName,
                errors: result.errors,
            });
            continue;
        }
        if (!result.changed)
            continue;
        repairedEvents += result.repairedEvents;
        sequenceRepairs += result.sequenceRepairs;
        pointerRepairs += result.pointerRepairs;
        rehashedEvents += result.rehashedEvents;
        repairedStreams.push({
            stream: streamName,
            repairedEvents: result.repairedEvents,
            sequenceRepairs: result.sequenceRepairs,
            pointerRepairs: result.pointerRepairs,
            rehashedEvents: result.rehashedEvents,
            backupPath: result.backupPath,
        });
    }
    const inspection = dryRun ? undefined : await inspectLedger(root);
    const index = dryRun ? undefined : await rebuildCoordinationIndex(root, { limit: options.limit });
    return {
        ok: skippedStreams.length === 0 && (inspection?.ok ?? true),
        root,
        dryRun,
        repairedEvents,
        sequenceRepairs,
        pointerRepairs,
        rehashedEvents,
        repairedStreams,
        skippedStreams,
        ...(inspection === undefined ? {} : { inspection }),
        ...(index === undefined ? {} : { index }),
        nextStep: dryRun
            ? "Re-run with dryRun=false or CLI --write after reviewing repairedStreams."
            : "Run coordination doctor/health. If conflicts remain, use coordination repair stale-claims on exact paths.",
    };
}

async function repairStream(streamPath, options) {
    const original = await readFile(streamPath, "utf8");
    const lines = original.split(/\r?\n/u).filter((line) => line.length > 0);
    const events = [];
    const errors = [];
    for (const [index, line] of lines.entries()) {
        try {
            events.push(JSON.parse(line));
        }
        catch (error) {
            errors.push({
                line: index + 1,
                message: `malformed event: ${error instanceof Error ? error.message : String(error)}`,
            });
        }
    }
    if (errors.length > 0) {
        return { changed: false, repairedEvents: 0, rehashedEvents: 0, errors };
    }
    let changed = false;
    let repairedEvents = 0;
    let rehashedEvents = 0;
    let previous = null;
    for (const [index, event] of events.entries()) {
        const originalHash = event.hash;
        const originalPrevEventId = event.prevEventId;
        const originalPrevHash = event.prevHash;
        const withoutHash = removeHash(event);
        const wireHash = hashForEvent(withoutHash);
        const extensionHash = hashForEventWithExtensions(withoutHash);
        const legacyCompatible = originalHash === wireHash;
        const extensionCompatible = originalHash === extensionHash;
        const extensionOnlyHash = hasExtensionMetadata(event) && !legacyCompatible && extensionCompatible;
        if (!legacyCompatible && !extensionCompatible) {
            errors.push({
                line: index + 1,
                eventId: event.id,
                message: "event hash is neither legacy-compatible nor known Enforcer extension-compatible",
            });
            break;
        }
        if (previous === null) {
            if (event.seq !== 1 || event.prevEventId !== null || event.prevHash !== null) {
                errors.push({
                    line: index + 1,
                    eventId: event.id,
                    message: "first event does not start a stream chain",
                });
                break;
            }
        }
        else if (event.prevEventId !== previous.id || event.prevHash !== previous.hash) {
            event.prevEventId = previous.id;
            event.prevHash = previous.hash;
            changed = true;
        }
        const nextHash = hashForEvent(removeHash(event));
        if (event.hash !== nextHash) {
            event.hash = nextHash;
            changed = true;
            if (extensionOnlyHash) {
                repairedEvents += 1;
            }
            else {
                rehashedEvents += 1;
            }
        }
        if (event.prevEventId !== originalPrevEventId || event.prevHash !== originalPrevHash) {
            rehashedEvents += 1;
        }
        previous = event;
    }
    if (errors.length > 0 || !changed) {
        return { changed: false, repairedEvents, rehashedEvents, errors };
    }
    const backupPath = `${streamPath}.legacy-hash-repair.${timestamp()}.bak`;
    if (!options.dryRun) {
        await copyFile(streamPath, backupPath);
        const tmpPath = `${streamPath}.legacy-hash-repair.tmp`;
        await writeFile(tmpPath, `${events.map((event) => JSON.stringify(event)).join("\n")}\n`, "utf8");
        await rename(tmpPath, streamPath);
    }
    return {
        changed,
        repairedEvents,
        rehashedEvents,
        errors,
        backupPath: options.dryRun ? null : backupPath,
    };
}

async function repairSequenceStream(streamPath, options) {
    const original = await readFile(streamPath, "utf8");
    const lines = original.split(/\r?\n/u).filter((line) => line.length > 0);
    const events = [];
    const errors = [];
    for (const [index, line] of lines.entries()) {
        try {
            events.push(JSON.parse(line));
        }
        catch (error) {
            errors.push({
                line: index + 1,
                message: `malformed event: ${error instanceof Error ? error.message : String(error)}`,
            });
        }
    }
    if (errors.length > 0) {
        return {
            changed: false,
            repairedEvents: 0,
            sequenceRepairs: 0,
            pointerRepairs: 0,
            rehashedEvents: 0,
            errors,
        };
    }
    let changed = false;
    let repairedEvents = 0;
    let sequenceRepairs = 0;
    let pointerRepairs = 0;
    let rehashedEvents = 0;
    let previous = null;
    for (const [index, event] of events.entries()) {
        let eventChanged = false;
        const originalHash = event.hash;
        const withoutHash = removeHash(event);
        const wireHash = hashForEvent(withoutHash);
        const extensionHash = hashForEventWithExtensions(withoutHash);
        if (originalHash !== wireHash && originalHash !== extensionHash) {
            errors.push({
                line: index + 1,
                eventId: event.id,
                message: "event hash is invalid; run coordination repair legacy-hash first if this is a context-hash stream",
            });
            break;
        }
        const expectedSeq = index + 1;
        if (event.seq !== expectedSeq) {
            event.seq = expectedSeq;
            sequenceRepairs += 1;
            eventChanged = true;
        }
        const expectedPrevEventId = previous?.id ?? null;
        const expectedPrevHash = previous?.hash ?? null;
        if (event.prevEventId !== expectedPrevEventId || event.prevHash !== expectedPrevHash) {
            event.prevEventId = expectedPrevEventId;
            event.prevHash = expectedPrevHash;
            pointerRepairs += 1;
            eventChanged = true;
        }
        const nextHash = hashForEvent(removeHash(event));
        if (event.hash !== nextHash) {
            event.hash = nextHash;
            rehashedEvents += 1;
            eventChanged = true;
        }
        if (eventChanged) {
            changed = true;
            repairedEvents += 1;
        }
        previous = event;
    }
    if (errors.length > 0 || !changed) {
        return {
            changed: false,
            repairedEvents,
            sequenceRepairs,
            pointerRepairs,
            rehashedEvents,
            errors,
        };
    }
    const backupPath = `${streamPath}.sequence-repair.${timestamp()}.bak`;
    if (!options.dryRun) {
        await copyFile(streamPath, backupPath);
        const tmpPath = `${streamPath}.sequence-repair.tmp`;
        await writeFile(tmpPath, `${events.map((event) => JSON.stringify(event)).join("\n")}\n`, "utf8");
        await rename(tmpPath, streamPath);
    }
    return {
        changed,
        repairedEvents,
        sequenceRepairs,
        pointerRepairs,
        rehashedEvents,
        errors,
        backupPath: options.dryRun ? null : backupPath,
    };
}

async function withStreamFileLock(streamPath, run) {
    const lockPath = streamPath.replace(/\.ndjson$/u, ".lock");
    let handle;
    try {
        handle = await open(lockPath, "wx");
        await handle.writeFile(String(process.pid));
    }
    catch (error) {
        return {
            changed: false,
            repairedEvents: 0,
            sequenceRepairs: 0,
            pointerRepairs: 0,
            rehashedEvents: 0,
            errors: [
                {
                    line: 0,
                    message: `stream lock is active at ${lockPath}: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
        };
    }
    finally {
        await handle?.close();
    }
    try {
        return await run();
    }
    finally {
        await rm(lockPath, { force: true });
    }
}

async function streamFiles(root) {
    try {
        await mkdir(streamsDir(root), { recursive: true });
        return (await readdir(streamsDir(root)))
            .filter((name) => name.endsWith(".ndjson") && !name.includes(".conflict."))
            .sort();
    }
    catch {
        return [];
    }
}

function removeHash(event) {
    const { hash: _hash, ...rest } = event;
    return rest;
}

function hasExtensionMetadata(event) {
    return Object.prototype.hasOwnProperty.call(event, "context");
}

function timestamp() {
    return new Date().toISOString().replaceAll(/[:.]/gu, "-");
}
