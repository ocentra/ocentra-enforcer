import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { COORDINATION_MATERIALIZER_VERSION } from "./live-index.js";

export async function checkpointAuditState(root) {
    try {
        const index = JSON.parse(await readFile(join(root, "db", "coordination-index.json"), "utf8"));
        if (index.materializerVersion !== COORDINATION_MATERIALIZER_VERSION
            || !Array.isArray(index.liveStreams)
            || index.state === undefined
            || index.audit === undefined) {
            return "missing";
        }
        return index.audit.ok === true ? "trusted" : "invalid";
    }
    catch (error) {
        if (isMissingPath(error) || error instanceof SyntaxError) {
            return "missing";
        }
        throw error;
    }
}

function isMissingPath(error) {
    return typeof error === "object" && error !== null && "code" in error && error.code === "ENOENT";
}
