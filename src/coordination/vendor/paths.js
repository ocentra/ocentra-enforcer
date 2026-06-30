import { join } from "node:path";
import { writerId } from "./domain.js";
export function streamsDir(root) {
    return join(root, "streams");
}
export function archiveStreamsDir(root) {
    return join(root, "archive", "streams");
}
export function archivedStreamDir(root, streamName) {
    return join(archiveStreamsDir(root), streamName);
}
export function viewsDir(root) {
    return join(root, "views");
}
export function laneViewsDir(root, lane) {
    return join(viewsDir(root), "lanes", lane);
}
export function streamPath(root, nodeId, lane) {
    return join(streamsDir(root), `${writerId(nodeId, lane)}.ndjson`);
}
export function lockPath(root, nodeId, lane) {
    return join(streamsDir(root), `${writerId(nodeId, lane)}.lock`);
}
export function writerFromStreamFile(fileName) {
    if (!fileName.endsWith(".ndjson")) {
        return undefined;
    }
    const writer = fileName.slice(0, -".ndjson".length);
    try {
        return writerIdFromRaw(writer);
    }
    catch {
        return undefined;
    }
}
function writerIdFromRaw(value) {
    const lastDot = value.lastIndexOf(".");
    if (lastDot <= 0 || lastDot === value.length - 1) {
        throw new Error(`invalid stream writer ${value}`);
    }
    return value;
}
