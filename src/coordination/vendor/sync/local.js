import { copyFile, mkdir, open, readdir, readFile } from "node:fs/promises";
import { join } from "node:path";
import { streamsDir } from "../paths.js";
export async function syncFromPeer(root, peer) {
    await mkdir(streamsDir(root), { recursive: true });
    let imported = 0;
    const conflicts = [];
    for (const fileName of await streamFiles(peer)) {
        const localPath = join(streamsDir(root), fileName);
        const peerPath = join(streamsDir(peer), fileName);
        const peerLines = await eventLines(join(streamsDir(peer), fileName));
        const localLines = await eventLines(localPath);
        if (!isPrefix(localLines, peerLines)) {
            const conflictName = conflictFileName(fileName);
            await copyFile(peerPath, join(streamsDir(root), conflictName));
            conflicts.push(conflictName);
            continue;
        }
        const handle = await open(localPath, "a");
        try {
            for (const line of peerLines.slice(localLines.length)) {
                await handle.appendFile(`${line}\n`);
                imported += 1;
            }
            await handle.sync();
        }
        finally {
            await handle.close();
        }
    }
    return { imported, transferredLines: imported, conflicts };
}
export async function streamFiles(root) {
    try {
        return (await readdir(streamsDir(root)))
            .filter((name) => name.endsWith(".ndjson") && !name.includes(".conflict."))
            .sort();
    }
    catch (error) {
        if (typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT") {
            return [];
        }
        throw error;
    }
}
export async function eventLines(path) {
    try {
        return (await readFile(path, "utf8"))
            .split(/\r?\n/)
            .map((line) => line.trim())
            .filter((line) => line.length > 0);
    }
    catch (error) {
        if (typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT") {
            return [];
        }
        throw error;
    }
}
export function isPrefix(localLines, peerLines) {
    if (localLines.length > peerLines.length) {
        return false;
    }
    return localLines.every((line, index) => line === peerLines[index]);
}
function conflictFileName(fileName) {
    return `${fileName}.conflict.${Date.now()}`;
}
