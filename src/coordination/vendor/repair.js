import { copyFile, open, readFile, rename, rm, writeFile } from "node:fs/promises";
import { inspectLedger } from "./doctor.js";
import { hashForEvent, hashForEventWithExtensions } from "./events.js";
import { rebuildCoordinationIndex } from "./read-index.js";
import { listCanonicalStreamNames, streamSegments } from "./stream.js";

export async function repairLegacyHashCompatibility(root, options = {}) {
    const dryRun = options.dryRun !== false;
    const streams = await listCanonicalStreamNames(root);
    const repairedStreams = [];
    const skippedStreams = [];
    let repairedEvents = 0;
    let rehashedEvents = 0;
    for (const streamName of streams) {
        const segments = await streamSegments(root, streamName);
        const result = dryRun
            ? await repairLegacyCanonicalStream(segments, { dryRun })
            : await withCanonicalStreamLocks(segments, () => repairLegacyCanonicalStream(segments, { dryRun }));
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
            backupPaths: result.backupPaths,
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
    const streams = await listCanonicalStreamNames(root);
    const repairedStreams = [];
    const skippedStreams = [];
    let repairedEvents = 0;
    let sequenceRepairs = 0;
    let pointerRepairs = 0;
    let rehashedEvents = 0;
    for (const streamName of streams) {
        const segments = await streamSegments(root, streamName);
        const result = dryRun
            ? await repairSequenceCanonicalStream(segments, { dryRun })
            : await withCanonicalStreamLocks(segments, () => repairSequenceCanonicalStream(segments, { dryRun }));
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
            backupPaths: result.backupPaths,
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

async function repairLegacyCanonicalStream(segments, options) {
    const segmentInputs = [];
    const errors = [];
    for (const segment of segments) {
        const original = await readFile(segment.path, "utf8");
        const lines = original.split(/\r?\n/u).filter((line) => line.length > 0);
        const events = [];
        for (const [index, line] of lines.entries()) {
            try {
                events.push(JSON.parse(line));
            }
            catch (error) {
                errors.push({
                    stream: segment.path,
                    line: index + 1,
                    message: `malformed event: ${error instanceof Error ? error.message : String(error)}`,
                });
            }
        }
        segmentInputs.push({ segment, events });
    }
    if (errors.length > 0) {
        return { changed: false, repairedEvents: 0, rehashedEvents: 0, errors };
    }
    let changed = false;
    let repairedEvents = 0;
    let rehashedEvents = 0;
    let previous = null;
    for (const segmentInput of segmentInputs) {
        for (const [index, event] of segmentInput.events.entries()) {
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
                    stream: segmentInput.segment.path,
                    line: index + 1,
                    eventId: event.id,
                    message: "event hash is neither legacy-compatible nor known Enforcer extension-compatible",
                });
                break;
            }
            if (previous === null) {
                if (event.seq !== 1 || event.prevEventId !== null || event.prevHash !== null) {
                    errors.push({
                        stream: segmentInput.segment.path,
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
        if (errors.length > 0) {
            break;
        }
    }
    if (errors.length > 0 || !changed) {
        return { changed: false, repairedEvents, rehashedEvents, errors };
    }
    const backupPaths = [];
    if (!options.dryRun) {
        for (const segmentInput of segmentInputs) {
            const streamPath = segmentInput.segment.path;
            const backupPath = `${streamPath}.legacy-hash-repair.${timestamp()}.bak`;
            backupPaths.push(backupPath);
            await copyFile(streamPath, backupPath);
            const tmpPath = `${streamPath}.legacy-hash-repair.tmp`;
            await writeFile(tmpPath, `${segmentInput.events.map((event) => JSON.stringify(event)).join("\n")}\n`, "utf8");
            await rename(tmpPath, streamPath);
        }
    }
    return {
        changed,
        repairedEvents,
        rehashedEvents,
        errors,
        backupPaths: options.dryRun ? [] : backupPaths,
    };
}

async function repairSequenceCanonicalStream(segments, options) {
    const segmentInputs = [];
    const errors = [];
    for (const segment of segments) {
        const original = await readFile(segment.path, "utf8");
        const lines = original.split(/\r?\n/u).filter((line) => line.length > 0);
        const events = [];
        for (const [index, line] of lines.entries()) {
            try {
                events.push(JSON.parse(line));
            }
            catch (error) {
                errors.push({
                    stream: segment.path,
                    line: index + 1,
                    message: `malformed event: ${error instanceof Error ? error.message : String(error)}`,
                });
            }
        }
        segmentInputs.push({ segment, events });
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
    let globalIndex = 0;
    for (const segmentInput of segmentInputs) {
        for (const [index, event] of segmentInput.events.entries()) {
            let eventChanged = false;
            const originalHash = event.hash;
            const withoutHash = removeHash(event);
            const wireHash = hashForEvent(withoutHash);
            const extensionHash = hashForEventWithExtensions(withoutHash);
            if (originalHash !== wireHash && originalHash !== extensionHash) {
                errors.push({
                    stream: segmentInput.segment.path,
                    line: index + 1,
                    eventId: event.id,
                    message: "event hash is invalid; run coordination repair legacy-hash first if this is a context-hash stream",
                });
                break;
            }
            const expectedSeq = globalIndex + 1;
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
            globalIndex += 1;
        }
        if (errors.length > 0) {
            break;
        }
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
    const backupPaths = [];
    if (!options.dryRun) {
        for (const segmentInput of segmentInputs) {
            const streamPath = segmentInput.segment.path;
            const backupPath = `${streamPath}.sequence-repair.${timestamp()}.bak`;
            backupPaths.push(backupPath);
            await copyFile(streamPath, backupPath);
            const tmpPath = `${streamPath}.sequence-repair.tmp`;
            await writeFile(tmpPath, `${segmentInput.events.map((event) => JSON.stringify(event)).join("\n")}\n`, "utf8");
            await rename(tmpPath, streamPath);
        }
    }
    return {
        changed,
        repairedEvents,
        sequenceRepairs,
        pointerRepairs,
        rehashedEvents,
        errors,
        backupPaths: options.dryRun ? [] : backupPaths,
    };
}

async function withCanonicalStreamLocks(segments, run) {
    const handles = [];
    const lockPaths = [];
    try {
        for (const segment of segments) {
            const lockPath = segment.path.replace(/\.ndjson$/u, ".lock");
            const handle = await open(lockPath, "wx");
            handles.push(handle);
            lockPaths.push(lockPath);
            await handle.writeFile(String(process.pid));
            await handle.close();
        }
    }
    catch (error) {
        for (const handle of handles) {
            await handle.close().catch(() => {});
        }
        for (const lockPath of lockPaths) {
            await rm(lockPath, { force: true });
        }
        return {
            changed: false,
            repairedEvents: 0,
            sequenceRepairs: 0,
            pointerRepairs: 0,
            rehashedEvents: 0,
            errors: [
                {
                    line: 0,
                    message: `stream lock is active: ${error instanceof Error ? error.message : String(error)}`,
                },
            ],
        };
    }
    try {
        return await run();
    }
    finally {
        await Promise.all(lockPaths.map((lockPath) => rm(lockPath, { force: true })));
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
