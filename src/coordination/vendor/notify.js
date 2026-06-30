import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { parseLaneId } from "./domain.js";
import { materialize } from "./materialize.js";
export async function notify(options) {
    const state = await materialize(options.root);
    const notifierStatePath = resolveStatePath(options);
    const notifierState = await readNotifierState(notifierStatePath);
    const wakeRequests = collectWakeRequests(state, options.lane).filter((request) => notifierState.seen[request.key] === undefined);
    if (options.peek !== true) {
        await writeNotifierState(notifierStatePath, markSeen(notifierState, wakeRequests));
    }
    return {
        ok: true,
        targetLane: options.lane,
        checkedAt: new Date().toISOString(),
        wakeRequests,
    };
}
export function collectWakeRequests(materialized, targetLane) {
    const inboxRequests = collectInboxWakeRequests(materialized, targetLane);
    const reportRequests = targetLane === "primary" ? collectPrimaryReportWakeRequests(materialized) : [];
    return [...inboxRequests, ...reportRequests].sort((left, right) => left.key.localeCompare(right.key));
}
function collectInboxWakeRequests(materialized, targetLane) {
    const inbox = laneView(materialized, targetLane)?.inbox;
    if (!Array.isArray(inbox)) {
        return [];
    }
    return inbox
        .filter((item) => typeof item.id === "string" && item.ackedBy.length === 0)
        .map((item) => {
        const summary = firstLine(item.body ?? "Ledger message");
        return {
            key: `inbox:${targetLane}:${item.id}`,
            targetLane,
            sourceLane: sourceLaneFromWriter(item.from) ?? "unknown",
            reason: "inbox",
            severity: "normal",
            summary,
            eventId: item.id,
        };
    });
}
function collectPrimaryReportWakeRequests(materialized) {
    const requests = [];
    for (const worker of workerValues(materialized)) {
        if (worker.lane === "primary") {
            continue;
        }
        const summary = firstLine(worker.summary ?? "");
        const reason = handoffReason(summary);
        if (reason === undefined) {
            continue;
        }
        requests.push({
            key: `report:${worker.writer}:${worker.lastSeenAt}:${summary}`,
            targetLane: parseLaneId("primary"),
            sourceLane: worker.lane,
            reason,
            severity: reason === "blocked" ? "high" : "normal",
            summary,
            eventId: worker.lastSeenAt,
        });
    }
    return requests;
}
function handoffReason(summary) {
    if (/^PR[-_ ]?READY\b/iu.test(summary)) {
        return "pr-ready";
    }
    if (/^DONE\b/iu.test(summary)) {
        return "done";
    }
    if (/^BLOCKED\b/iu.test(summary)) {
        return "blocked";
    }
    return undefined;
}
function laneView(materialized, lane) {
    if (isReadonlyMap(materialized.lanes)) {
        return materialized.lanes.get(lane);
    }
    return materialized.lanes[lane];
}
function workerValues(materialized) {
    if (isReadonlyMap(materialized.workers)) {
        return Array.from(materialized.workers.values());
    }
    return Object.values(materialized.workers);
}
function isReadonlyMap(value) {
    return value instanceof Map;
}
function resolveStatePath(options) {
    if (options.stateFile !== undefined) {
        return resolve(options.stateFile);
    }
    return join(options.root, "notifier", `${options.lane}.json`);
}
async function readNotifierState(filePath) {
    try {
        const parsed = JSON.parse(await readFile(filePath, "utf8"));
        const seen = parsed !== null && typeof parsed.seen === "object" && parsed.seen !== null ? parsed.seen : {};
        return {
            seen,
            ...(parsed.updatedAt === undefined ? {} : { updatedAt: parsed.updatedAt }),
        };
    }
    catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return { seen: {} };
        }
        throw error;
    }
}
async function writeNotifierState(filePath, state) {
    await mkdir(dirname(filePath), { recursive: true });
    await writeFile(filePath, `${JSON.stringify(state, null, 2)}\n`);
}
function markSeen(state, wakeRequests) {
    const seen = { ...state.seen };
    const now = new Date().toISOString();
    for (const request of wakeRequests) {
        seen[request.key] = now;
    }
    return { seen, updatedAt: now };
}
function firstLine(value) {
    return value.split(/\r?\n/u)[0]?.trim() ?? "";
}
function sourceLaneFromWriter(writer) {
    return writer.split(".").at(-1);
}
