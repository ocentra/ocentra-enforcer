import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { getActiveTasks, getWorkers, materialize } from "./materialize.js";
import { buildStreamManifest } from "./manifest.js";
import { loadPeerRegistry } from "./peers.js";
import { buildPresenceMatrix } from "./presence.js";
import { viewsDir } from "./paths.js";

export async function rebuildCoordinationIndex(root, options = {}) {
    const state = await materialize(root);
    const presence = buildPresenceMatrix(root, state, options);
    const streams = await buildStreamManifest(root);
    const peers = await loadPeerRegistry(root);
    const dbRoot = join(root, "db");
    await mkdir(dbRoot, { recursive: true });
    await mkdir(viewsDir(root), { recursive: true });
    const index = {
        ok: true,
        root,
        generatedAt: new Date().toISOString(),
        backend: "json",
        dashboard: state.dashboard,
        presence,
        ownership: state.ownership,
        workers: getWorkers(state),
        activeTasks: getActiveTasks(state),
        peers,
        streams,
    };
    await writeFile(join(viewsDir(root), "presence.json"), `${JSON.stringify(presence, null, 2)}\n`);
    await writeFile(join(viewsDir(root), "streams-manifest.json"), `${JSON.stringify(streams, null, 2)}\n`);
    await writeFile(join(dbRoot, "coordination-index.json"), `${JSON.stringify(index, null, 2)}\n`);
    const sqlite = await writeOptionalSqliteIndex(root, presence, streams);
    const status = {
        ok: true,
        root,
        generatedAt: index.generatedAt,
        jsonIndex: "db/coordination-index.json",
        sqlite,
        counts: {
            presenceRows: presence.totalRows,
            streams: streams.streams.length,
            activeClaims: state.ownership.activeClaims.length,
            workers: index.workers.length,
            activeTasks: index.activeTasks.length,
        },
    };
    await writeFile(join(dbRoot, "coordination-index-status.json"), `${JSON.stringify(status, null, 2)}\n`);
    return status;
}

async function writeOptionalSqliteIndex(root, presence, streams) {
    const database = "db/coordination.sqlite";
    if (process.env.OCENTRA_COORDINATION_SQLITE !== "1") {
        return {
            available: false,
            database,
            detail: "SQLite hot index is opt-in; set OCENTRA_COORDINATION_SQLITE=1. JSON views are the default cross-platform fallback.",
        };
    }
    try {
        const sqlite = await import("node:sqlite");
        if (typeof sqlite.DatabaseSync !== "function") {
            return { available: false, database, detail: "node:sqlite DatabaseSync is unavailable in this Node runtime." };
        }
        const db = new sqlite.DatabaseSync(join(root, database));
        db.exec("CREATE TABLE IF NOT EXISTS presence (writer TEXT PRIMARY KEY, lane TEXT, machine TEXT, project_id TEXT, worktree_root TEXT, thread_id TEXT, state TEXT, last_seen_at TEXT)");
        db.exec("CREATE TABLE IF NOT EXISTS streams (stream TEXT PRIMARY KEY, writer TEXT, event_count INTEGER, byte_length INTEGER, tail_hash TEXT)");
        db.exec("DELETE FROM presence");
        db.exec("DELETE FROM streams");
        const insertPresence = db.prepare("INSERT INTO presence VALUES (?, ?, ?, ?, ?, ?, ?, ?)");
        for (const row of presence.rows) {
            insertPresence.run(row.writer, row.lane, row.machine, row.projectId, row.worktreeRoot, row.codexThreadId, row.state, row.lastSeenAt);
        }
        const insertStream = db.prepare("INSERT INTO streams VALUES (?, ?, ?, ?, ?)");
        for (const stream of streams.streams) {
            insertStream.run(stream.stream, stream.writer, stream.eventCount, stream.byteLength, stream.tailHash);
        }
        db.close();
        return { available: true, database, detail: "SQLite hot read index rebuilt from canonical streams." };
    }
    catch (error) {
        return {
            available: false,
            database,
            detail: `SQLite index unavailable; JSON views remain authoritative fallback: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}
