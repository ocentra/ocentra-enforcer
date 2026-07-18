import { checkpointAuditState } from "./append-checkpoint.js";

export async function readAppendTail(root, readers) {
    const checkpoint = await checkpointAuditState(root);
    if (checkpoint === "invalid") {
        throw new Error("ledger checkpoint audit failed; rebuild or repair before appending events");
    }
    return checkpoint === "trusted" ? readers.readLive() : readers.readCanonical();
}
