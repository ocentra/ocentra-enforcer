import { createHash } from "node:crypto";
import { Schema } from "effect";
import { ClaimPathSchema, EventHashSchema, EventIdSchema, EventTypeSchema, HubIdSchema, IsoTimestampSchema, LaneIdSchema, MessageAddressSchema, NodeIdSchema, NodeNameSchema, PullRequestUrlSchema, SessionIdSchema, StatusStateSchema, TaskIdSchema, TaskStateSchema, UserTextSchema, WorkerStateSchema, WriterIdSchema, parseEventHash, parseEventId, } from "./domain.js";
export const HubEventSchema = Schema.Struct({
    id: EventIdSchema,
    schema: Schema.Literal(1),
    hub: HubIdSchema,
    nodeId: NodeIdSchema,
    nodeName: NodeNameSchema,
    lane: LaneIdSchema,
    writer: WriterIdSchema,
    type: EventTypeSchema,
    ts: IsoTimestampSchema,
    seq: Schema.Number,
    prevEventId: Schema.Union(EventIdSchema, Schema.Null),
    prevHash: Schema.Union(EventHashSchema, Schema.Null),
    hash: EventHashSchema,
    to: Schema.optional(MessageAddressSchema),
    body: Schema.optional(UserTextSchema),
    messageId: Schema.optional(EventIdSchema),
    paths: Schema.optional(Schema.Array(ClaimPathSchema)),
    reason: Schema.optional(UserTextSchema),
    owner: Schema.optional(WriterIdSchema),
    owners: Schema.optional(Schema.Array(WriterIdSchema)),
    state: Schema.optional(StatusStateSchema),
    workerState: Schema.optional(WorkerStateSchema),
    taskId: Schema.optional(TaskIdSchema),
    taskState: Schema.optional(TaskStateSchema),
    title: Schema.optional(UserTextSchema),
    prUrl: Schema.optional(PullRequestUrlSchema),
    summary: Schema.optional(UserTextSchema),
    ttlSeconds: Schema.optional(Schema.Number),
    sessionId: Schema.optional(SessionIdSchema),
    context: Schema.optional(Schema.Unknown),
});
const decodeHubEvent = Schema.decodeUnknownSync(HubEventSchema);
export function parseHubEvent(input) {
    return decodeHubEvent(input);
}
export function completeEvent(input, streamTip, rawEventId) {
    const eventWithoutHash = {
        ...input,
        id: parseEventId(rawEventId),
        seq: streamTip === undefined ? 1 : streamTip.seq + 1,
        prevEventId: streamTip?.id ?? null,
        prevHash: streamTip?.hash ?? null,
    };
    return decodeHubEvent({
        ...eventWithoutHash,
        hash: hashForEvent(eventWithoutHash),
    });
}
export function assertCoordinationHashCompatibility() {
    const result = coordinationHashCompatibility();
    if (!result.ok) {
        throw new Error(`coordination hash compatibility check failed: expected ${result.expectedWireHash}, got ${result.actualWireHash}`);
    }
}
export function coordinationHashCompatibility() {
    const actualWireHash = hashForEvent(HASH_COMPATIBILITY_SAMPLE_EVENT);
    const extensionHash = hashForEventWithExtensions(HASH_COMPATIBILITY_SAMPLE_EVENT);
    return {
        ok: actualWireHash === EXPECTED_HASH_COMPATIBILITY_WIRE_HASH &&
            extensionHash !== EXPECTED_HASH_COMPATIBILITY_WIRE_HASH,
        expectedWireHash: EXPECTED_HASH_COMPATIBILITY_WIRE_HASH,
        actualWireHash,
        extensionHash,
        contextExcludedFromWireHash: extensionHash !== actualWireHash,
    };
}
export function assertEventHash(event) {
    const expected = hashForEvent(withoutHash(event));
    if (event.hash !== expected) {
        throw new Error(`event ${event.id} hash mismatch`);
    }
}
export function hashForEvent(event) {
    const serialized = JSON.stringify(canonicalize(wireHashEvent(event)));
    const digest = createHash("sha256").update(serialized).digest("hex");
    return parseEventHash(`sha256:${digest}`);
}
export function hashForEventWithExtensions(event) {
    const serialized = JSON.stringify(canonicalize(event));
    const digest = createHash("sha256").update(serialized).digest("hex");
    return parseEventHash(`sha256:${digest}`);
}
export function eventType(value) {
    return value;
}
export function targetLane(event) {
    return event.lane;
}
function withoutHash(event) {
    const { hash: _hash, ...rest } = event;
    return rest;
}
const WIRE_HASH_FIELDS = new Set([
    "id",
    "schema",
    "hub",
    "nodeId",
    "nodeName",
    "lane",
    "writer",
    "type",
    "ts",
    "seq",
    "prevEventId",
    "prevHash",
    "to",
    "body",
    "messageId",
    "paths",
    "reason",
    "owner",
    "owners",
    "state",
    "workerState",
    "taskId",
    "taskState",
    "title",
    "prUrl",
    "summary",
    "ttlSeconds",
    "sessionId",
]);
const EXPECTED_HASH_COMPATIBILITY_WIRE_HASH = "sha256:c4333184613bd63a0d2918e3be4c88ce2ea4a32d9fc7e07bb755ade688aada76";
const HASH_COMPATIBILITY_SAMPLE_EVENT = {
    id: "evt_compatibility0000000000000000000000",
    schema: 1,
    hub: "compat-hub",
    nodeId: "node_compatibility",
    nodeName: "CompatNode",
    lane: "codex-a",
    writer: "node_compatibility.codex-a",
    type: "claim",
    ts: "2026-06-30T00:00:00.000Z",
    seq: 1,
    prevEventId: null,
    prevHash: null,
    paths: ["src/lib.rs"],
    reason: "compatibility sentinel",
    context: {
        projectId: "must-not-affect-wire-hash",
        repoRoot: "C:/repo",
    },
};
function wireHashEvent(event) {
    return Object.fromEntries(Object.entries(event).filter(([key, value]) => WIRE_HASH_FIELDS.has(key) && value !== undefined));
}
function canonicalize(value) {
    if (Array.isArray(value)) {
        return value.map(canonicalize);
    }
    if (value !== null && typeof value === "object") {
        return Object.fromEntries(Object.entries(value)
            .filter((entry) => entry[1] !== undefined)
            .sort(([left], [right]) => left.localeCompare(right))
            .map(([key, entryValue]) => [key, canonicalize(entryValue)]));
    }
    return value;
}
