import { randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { hostname } from "node:os";
import { join } from "node:path";
import { nowIso, parseHubConfig, parseHubId, parseLaneId, parseNodeId, parseNodeName, } from "./domain.js";
export function identityDir(root) {
    return join(root, "identity");
}
export function identityPath(root) {
    return join(identityDir(root), "node.json");
}
export async function initIdentity(input) {
    const config = {
        hub: parseHubId(input.hub),
        nodeId: parseNodeId(input.nodeId ?? randomNodeId()),
        nodeName: parseNodeName(input.nodeName ?? displayHostname()),
        defaultLane: parseLaneId(input.lane),
        createdAt: nowIso(),
    };
    await mkdir(identityDir(input.root), { recursive: true });
    await writeFile(identityPath(input.root), `${JSON.stringify(config, null, 2)}\n`, {
        flag: "wx",
    });
    return config;
}
export async function loadIdentity(root) {
    const raw = await readFile(identityPath(root), "utf8");
    return parseHubConfig(JSON.parse(raw));
}
export function randomEventId() {
    return `evt_${randomUUID().replaceAll("-", "")}`;
}
function randomNodeId() {
    return parseNodeId(`node_${randomUUID().replaceAll("-", "")}`);
}
function displayHostname() {
    return parseNodeName(hostname().replaceAll(/[^A-Za-z0-9._-]/g, "_"));
}
export function resolveHubId(config) {
    return config.hub;
}
export function resolveNodeId(config) {
    return config.nodeId;
}
export function resolveNodeName(config) {
    return config.nodeName;
}
export function resolveLane(config, lane) {
    return lane === undefined ? config.defaultLane : parseLaneId(lane);
}
