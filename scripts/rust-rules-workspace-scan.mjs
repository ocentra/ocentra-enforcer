import { MessageChannel, Worker, receiveMessageOnPort } from "node:worker_threads";
import { scanRustFile } from "./rust-rules-source-scan.mjs";
import {
  balancedWorkspacePartitions,
  workspaceWorkerCount,
} from "./rust-rules-workspace-partitioning.mjs";

const WORKSPACE_WORKER_TIMEOUT_MS = 600_000;
// Node worker startup/import cost is material on Windows.  Spawning a worker
// for a small scope is slower than the serial scanner and makes the narrow
// regression fixtures look like a workspace timeout.  Workspace scans still
// use workers once the file set is large enough to amortize that fixed cost.
const MIN_PARALLEL_FILE_COUNT = 64;

/** Collects Rust findings serially for an explicit file list. */
export function serialRustFileFindings(root, config, files) {
  const proofEvidenceCache = new Map();
  return files.map((filePath, index) => ({
    index,
    findings: scanRustFile(root, filePath, config, { proofEvidenceCache }),
  }));
}

function startWorkspaceWorkers(root, config, partitions) {
  return partitions.map((partition) => {
    const { port1, port2 } = new MessageChannel();
    const worker = new Worker(new URL("./rust-rules-workspace-worker.mjs", import.meta.url), {
      workerData: { root, config, entries: partition.entries, port: port2 },
      transferList: [port2],
    });
    return { worker, port: port1, result: null };
  });
}

function receiveWorkspaceResults(workers) {
  const sleeper = new Int32Array(new SharedArrayBuffer(Int32Array.BYTES_PER_ELEMENT));
  const deadline = Date.now() + WORKSPACE_WORKER_TIMEOUT_MS;
  let remaining = workers.length;
  while (remaining > 0) {
    for (const state of workers) {
      if (state.result !== null) continue;
      const received = receiveMessageOnPort(state.port);
      if (!received) continue;
      state.result = received.message;
      remaining -= 1;
    }
    if (remaining === 0) break;
    if (Date.now() >= deadline) {
      for (const state of workers) void state.worker.terminate();
      throw new Error(`Workspace Rust scan workers exceeded ${WORKSPACE_WORKER_TIMEOUT_MS}ms.`);
    }
    Atomics.wait(sleeper, 0, 0, 10);
  }
}

function collectWorkspaceResults(workers) {
  const indexed = [];
  for (const state of workers) {
    state.port.close();
    void state.worker.terminate();
    if (!state.result?.ok) {
      throw new Error(state.result?.error ?? "Workspace Rust scan worker failed.");
    }
    indexed.push(...state.result.results);
  }
  indexed.sort((left, right) => left.index - right.index);
  return indexed;
}

/** Collects Rust findings in parallel for an explicit file list. */
export function parallelRustFileFindings(root, config, files, requestedWorkerCount) {
  const workerCount = workspaceWorkerCount(files.length, requestedWorkerCount);
  if (workerCount <= 1 || files.length < MIN_PARALLEL_FILE_COUNT) {
    return serialRustFileFindings(root, config, files);
  }
  const partitions = balancedWorkspacePartitions(root, files, workerCount);
  const workers = startWorkspaceWorkers(root, config, partitions);
  receiveWorkspaceResults(workers);
  return collectWorkspaceResults(workers);
}

/** Collects Rust findings for a scan scope using configured workers. */
export function rustFileFindings(root, config, scope, options = {}) {
  const useParallel =
    scope.mode === "all" && !config.failFast && !options.forceSerial && scope.files.length >= 32;
  return useParallel
    ? parallelRustFileFindings(root, config, scope.files, options.workerCount)
    : serialRustFileFindings(root, config, scope.files);
}
