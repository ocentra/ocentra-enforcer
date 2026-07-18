import { createHash } from "node:crypto";

const DIGEST_BUFFER_BYTES = 64 * 1024;

export async function prefixDigest(handle, end) {
    const hash = createHash("sha256");
    const buffer = Buffer.alloc(Math.min(DIGEST_BUFFER_BYTES, end));
    let position = 0;
    while (position < end) {
        const { bytesRead } = await handle.read(buffer, 0, Math.min(buffer.length, end - position), position);
        if (bytesRead === 0) {
            const error = new Error("stream changed while reading checkpoint prefix");
            error.code = "ENOENT";
            throw error;
        }
        hash.update(buffer.subarray(0, bytesRead));
        position += bytesRead;
    }
    return `sha256:${hash.digest("hex")}`;
}

export function samePrefixDigest(left, right) {
    return typeof left === "string" && left === right;
}
