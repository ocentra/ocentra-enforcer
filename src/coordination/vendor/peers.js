import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { parsePeerName, parsePeerUrl, parseUserText, } from "./domain.js";
const registryFileName = "peers.json";
export async function addPeer(root, input) {
    const peer = {
        name: parsePeerName(input.name),
        url: parsePeerUrl(input.url),
        mode: parsePeerMode(input.mode ?? "pull"),
        ...(input.tokenEnv === undefined ? {} : { tokenEnv: parseUserText(input.tokenEnv) }),
    };
    const registry = await loadPeerRegistry(root);
    const peers = [
        ...registry.peers.filter((existing) => existing.name !== peer.name),
        peer,
    ].sort((left, right) => left.name.localeCompare(right.name));
    const next = { peers };
    await savePeerRegistry(root, next);
    return next;
}
export async function loadPeerRegistry(root) {
    try {
        const raw = JSON.parse(await readFile(registryPath(root), "utf8"));
        if (!isRegistryShape(raw)) {
            throw new Error("peers.json must contain { peers: [...] }");
        }
        return {
            peers: raw.peers.map((peer) => ({
                name: parsePeerName(peer.name),
                url: parsePeerUrl(peer.url),
                mode: parsePeerMode(peer.mode ?? "pull"),
                ...(peer.tokenEnv === undefined ? {} : { tokenEnv: parseUserText(peer.tokenEnv) }),
            })),
        };
    }
    catch (error) {
        if (error instanceof Error && "code" in error && error.code === "ENOENT") {
            return { peers: [] };
        }
        throw error;
    }
}
export async function removePeer(root, name) {
    const registry = await loadPeerRegistry(root);
    const peerName = parsePeerName(name);
    const peers = registry.peers.filter((peer) => peer.name !== peerName);
    const next = { peers };
    await savePeerRegistry(root, next);
    return next;
}
export async function resolvePeer(root, peerOrUrl) {
    if (peerOrUrl.startsWith("http://") || peerOrUrl.startsWith("https://")) {
        return { url: parsePeerUrl(peerOrUrl) };
    }
    const name = parsePeerName(peerOrUrl);
    const registry = await loadPeerRegistry(root);
    const peer = registry.peers.find((candidate) => candidate.name === name);
    if (peer === undefined) {
        throw new Error(`unknown peer alias: ${name}`);
    }
    const token = peer.tokenEnv === undefined ? undefined : process.env[peer.tokenEnv];
    return {
        name,
        url: peer.url,
        ...(token === undefined ? {} : { token }),
    };
}
async function savePeerRegistry(root, registry) {
    const path = registryPath(root);
    await mkdir(dirname(path), { recursive: true });
    await writeFile(path, `${JSON.stringify(registry, null, 2)}\n`);
}
function registryPath(root) {
    return join(root, registryFileName);
}
function isRegistryShape(raw) {
    return typeof raw === "object"
        && raw !== null
        && "peers" in raw
        && Array.isArray(raw.peers)
        && raw.peers.every((peer) => (typeof peer === "object"
            && peer !== null
            && "name" in peer
            && typeof peer.name === "string"
            && "url" in peer
            && typeof peer.url === "string"
            && (!("mode" in peer) || typeof peer.mode === "string")
            && (!("tokenEnv" in peer) || typeof peer.tokenEnv === "string")));
}
function parsePeerMode(value) {
    if (value === "pull" || value === "push" || value === "both") return value;
    throw new Error("peer mode must be pull, push, or both");
}
