import { Schema } from "effect";
const identityPattern = /^[A-Za-z0-9._-]+$/;
const writerPattern = /^[A-Za-z0-9._-]+\.[A-Za-z0-9._-]+$/;
const eventHashPattern = /^sha256:[a-f0-9]{64}$/;
export const HubIdSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(80), Schema.pattern(identityPattern), Schema.brand("HubId"));
export const NodeIdSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(96), Schema.pattern(identityPattern), Schema.brand("NodeId"));
export const NodeNameSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(96), Schema.pattern(identityPattern), Schema.brand("NodeName"));
export const LaneIdSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(80), Schema.pattern(identityPattern), Schema.brand("LaneId"));
export const WriterIdSchema = Schema.String.pipe(Schema.minLength(3), Schema.maxLength(180), Schema.pattern(writerPattern), Schema.brand("WriterId"));
export const EventIdSchema = Schema.String.pipe(Schema.minLength(8), Schema.maxLength(80), Schema.pattern(identityPattern), Schema.brand("EventId"));
export const EventHashSchema = Schema.String.pipe(Schema.pattern(eventHashPattern), Schema.brand("EventHash"));
export const IsoTimestampSchema = Schema.String.pipe(Schema.minLength(20), Schema.brand("IsoTimestamp"));
export const ClaimPathSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(260), Schema.brand("ClaimPath"));
export const MessageAddressSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(120), Schema.brand("MessageAddress"));
export const UserTextSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(4000), Schema.brand("UserText"));
export const TaskIdSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(120), Schema.pattern(identityPattern), Schema.brand("TaskId"));
export const SessionIdSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(180), Schema.pattern(identityPattern), Schema.brand("SessionId"));
export const PullRequestUrlSchema = Schema.String.pipe(Schema.minLength(8), Schema.maxLength(500), Schema.pattern(/^https?:\/\/.+/), Schema.brand("PullRequestUrl"));
export const PeerNameSchema = Schema.String.pipe(Schema.minLength(1), Schema.maxLength(80), Schema.pattern(identityPattern), Schema.brand("PeerName"));
export const PeerUrlSchema = Schema.String.pipe(Schema.minLength(8), Schema.maxLength(500), Schema.pattern(/^https?:\/\/.+/), Schema.brand("PeerUrl"));
export const WorkerStateSchema = Schema.Literal("idle", "started", "progress", "working", "blocked", "pr_ready", "done", "offline").pipe(Schema.brand("WorkerState"));
export const TaskStateSchema = Schema.Literal("queued", "started", "progress", "blocked", "pr_ready", "done", "cancelled").pipe(Schema.brand("TaskState"));
export const StatusStateSchema = Schema.Literal("idle", "working", "blocked", "ready", "done", "handoff", "online").pipe(Schema.brand("StatusState"));
export const EventTypeSchema = Schema.Literal("lane.register", "message", "ack", "claim", "release", "claim.resolve", "editIntent", "status", "heartbeat", "session.claim", "session.release", "worker.update", "task.update", "report", "handoff", "note").pipe(Schema.brand("EventType"));
export const HubConfigSchema = Schema.Struct({
    hub: HubIdSchema,
    nodeId: NodeIdSchema,
    nodeName: NodeNameSchema,
    defaultLane: LaneIdSchema,
    createdAt: IsoTimestampSchema,
});
export const parseHubId = Schema.decodeUnknownSync(HubIdSchema);
export const parseNodeId = Schema.decodeUnknownSync(NodeIdSchema);
export const parseNodeName = Schema.decodeUnknownSync(NodeNameSchema);
export const parseLaneId = Schema.decodeUnknownSync(LaneIdSchema);
export const parseWriterId = Schema.decodeUnknownSync(WriterIdSchema);
export const parseEventId = Schema.decodeUnknownSync(EventIdSchema);
export const parseEventHash = Schema.decodeUnknownSync(EventHashSchema);
export const parseIsoTimestamp = Schema.decodeUnknownSync(IsoTimestampSchema);
export const parseClaimPath = Schema.decodeUnknownSync(ClaimPathSchema);
export const parseMessageAddress = Schema.decodeUnknownSync(MessageAddressSchema);
export const parseUserText = Schema.decodeUnknownSync(UserTextSchema);
export const parseTaskId = Schema.decodeUnknownSync(TaskIdSchema);
export const parseSessionId = Schema.decodeUnknownSync(SessionIdSchema);
export const parsePullRequestUrl = Schema.decodeUnknownSync(PullRequestUrlSchema);
export const parsePeerName = Schema.decodeUnknownSync(PeerNameSchema);
export const parsePeerUrl = Schema.decodeUnknownSync(PeerUrlSchema);
export const parseWorkerState = Schema.decodeUnknownSync(WorkerStateSchema);
export const parseTaskState = Schema.decodeUnknownSync(TaskStateSchema);
export const parseStatusState = Schema.decodeUnknownSync(StatusStateSchema);
export const parseEventType = Schema.decodeUnknownSync(EventTypeSchema);
export const parseHubConfig = Schema.decodeUnknownSync(HubConfigSchema);
export function writerId(nodeId, lane) {
    return parseWriterId(`${nodeId}.${lane}`);
}
export function nowIso() {
    return parseIsoTimestamp(new Date().toISOString());
}
