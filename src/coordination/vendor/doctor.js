import { readFile } from "node:fs/promises";
import { basename } from "node:path";
import { assertEventHash, parseHubEvent } from "./events.js";
import { listCanonicalStreamNames, streamSegments } from "./stream.js";
export async function inspectLedger(root) {
    const diagnostics = [];
    for (const stream of await listCanonicalStreamNames(root)) {
        const events = [];
        for (const segment of await streamSegments(root, stream)) {
            const lines = await readLines(segment.path);
            for (const [index, line] of lines.entries()) {
                if (line.trim().length === 0) {
                    continue;
                }
                try {
                    const event = parseHubEvent(JSON.parse(line));
                    try {
                        assertEventHash(event);
                    }
                    catch (error) {
                        diagnostics.push({
                            level: "error",
                            stream: segment.archived ? `${stream}/${basename(segment.path)}` : stream,
                            line: index + 1,
                            message: `hash-invalid event: ${String(error)}`,
                        });
                        continue;
                    }
                    events.push(event);
                }
                catch (error) {
                    diagnostics.push({
                        level: index === lines.length - 1 ? "warning" : "error",
                        stream: segment.archived ? `${stream}/${basename(segment.path)}` : stream,
                        line: index + 1,
                        message: index === lines.length - 1
                            ? "ignored malformed final line"
                            : `malformed event: ${String(error)}`,
                    });
                }
            }
        }
        for (const [index, event] of events.entries()) {
            const previous = events[index - 1];
            if (previous === undefined) {
                if (event.seq !== 1 || event.prevEventId !== null || event.prevHash !== null) {
                    diagnostics.push({
                        level: "error",
                        stream,
                        line: index + 1,
                        message: "first event does not start a stream chain",
                    });
                }
                continue;
            }
            if (event.seq !== previous.seq + 1) {
                diagnostics.push({
                    level: "error",
                    stream,
                    line: index + 1,
                    message: `sequence break: expected ${previous.seq + 1}, got ${event.seq}`,
                });
            }
            if (event.prevEventId !== previous.id || event.prevHash !== previous.hash) {
                diagnostics.push({
                    level: "error",
                    stream,
                    line: index + 1,
                    message: "previous event pointer does not match stream tip",
                });
            }
        }
    }
    return {
        ok: diagnostics.every((diagnostic) => diagnostic.level !== "error"),
        diagnostics,
    };
}
async function readLines(path) {
    return (await readFile(path, "utf8")).split(/\r?\n/).filter((line) => line.length > 0);
}
