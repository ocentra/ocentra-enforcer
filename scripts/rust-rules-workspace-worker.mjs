import { parentPort, workerData } from "node:worker_threads";
import { scanRustFile } from "./rust-rules-source-scan.mjs";

const { root, config, entries, port } = workerData;

try {
  const proofEvidenceCache = new Map();
  const results = entries.map(({ index, filePath }) => ({
    index,
    findings: scanRustFile(root, filePath, config, { proofEvidenceCache }),
  }));
  port.postMessage({ ok: true, results });
} catch (error) {
  port.postMessage({
    ok: false,
    error: error instanceof Error ? error.stack ?? error.message : String(error),
  });
} finally {
  port.close();
  parentPort?.close();
}
