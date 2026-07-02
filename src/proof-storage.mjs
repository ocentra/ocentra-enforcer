import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";

import { normalizeRel, toPosix, uniqueSorted } from "./path-utils.mjs";

const PROOF_STORAGE_DIR = ".enforce/proofs";
const PROOF_MANIFEST = "db/proof-manifest.json";
const DEFAULT_PROOF_RETENTION = Object.freeze({
  maxRunsPerProof: 20,
  maxFailedRuns: 20,
  maxArtifactBytes: 50 * 1024 * 1024,
  pruneAfterDays: 14,
  pinPrReadyDays: 30,
});

const SECRET_REDACTION_PATTERNS = [
  /\bgh[pousr]_[A-Za-z0-9_]{20,}\b/gu,
  /\bAKIA[0-9A-Z]{16}\b/gu,
  /\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b/gu,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/gu,
  /\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{16,}["']/giu,
  /-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----/gu,
];

function proofHarnessConfig() {
  return {
    storageDir: PROOF_STORAGE_DIR,
    store: "ndjson-duckdb",
    maxArtifactBytes: 8000,
    maxRuns: null,
    maxRunsPerTool: null,
    maxFailedRuns: DEFAULT_PROOF_RETENTION.maxFailedRuns,
    pruneAfterDays: DEFAULT_PROOF_RETENTION.pruneAfterDays,
  };
}

function proofStorageRoot(root) {
  return path.join(root, PROOF_STORAGE_DIR);
}

function proofRunDir(root, runId) {
  return path.join(proofStorageRoot(root), "runs", runId);
}

function sanitizeSegment(value) {
  return (
    String(value ?? "proof")
      .replace(/[^A-Za-z0-9._-]+/gu, "-")
      .replace(/^-+|-+$/gu, "") || "proof"
  );
}

function createProofRunId(prefix) {
  const safePrefix = sanitizeSegment(prefix);
  const stamp = new Date().toISOString().replace(/[-:.TZ]/gu, "").slice(0, 14);
  return `${safePrefix}-${stamp}-${crypto.randomUUID().slice(0, 8)}`;
}

function sanitizeRelativeArtifactName(relPath) {
  return toPosix(relPath)
    .replace(/^[A-Za-z]:/u, "")
    .replace(/^\/+/u, "")
    .replace(/[^A-Za-z0-9._/-]+/gu, "-");
}

function redactSecrets(value) {
  let text = String(value ?? "");
  for (const pattern of SECRET_REDACTION_PATTERNS) text = text.replace(pattern, "[REDACTED]");
  return text;
}

function redactedJson(value) {
  return JSON.parse(redactSecrets(JSON.stringify(value)));
}

function gitState(root) {
  const commit = runGit(root, ["rev-parse", "HEAD"]);
  const branch = runGit(root, ["rev-parse", "--abbrev-ref", "HEAD"]);
  const status = runGit(root, ["status", "--porcelain"]);
  return {
    commit,
    branch,
    dirty: status === null ? null : status.length > 0,
  };
}

function runGit(root, args) {
  try {
    return fs.existsSync(path.join(root, ".git")) ? childProcessGit(root, args) : null;
  } catch {
    return null;
  }
}

function childProcessGit(root, args) {
  const result = spawnSync("git", args, { cwd: root, encoding: "utf8", shell: false });
  if ((result.status ?? 1) !== 0) return null;
  return result.stdout.trim();
}

function writeNdjson(filePath, rows) {
  fs.writeFileSync(
    filePath,
    rows.map((row) => JSON.stringify(row)).join("\n") + (rows.length > 0 ? "\n" : ""),
    "utf8",
  );
}

function baseProofRun({
  root: _root,
  runId,
  proofContext,
  status,
  exitCode,
  startedAt,
  endedAt,
  command,
  diagnosticCount,
  pinned = false,
}) {
  return {
    ...proofContext,
    runId,
    status,
    ok: status === "passed",
    exitCode,
    startedAt,
    endedAt,
    command,
    diagnosticCount,
    pinned,
    retention: DEFAULT_PROOF_RETENTION,
    artifacts: [],
  };
}

function writeManualProofRun({ root, runId, proofContext, status, message }) {
  const startedAt = new Date().toISOString();
  const runDir = proofRunDir(root, runId);
  fs.mkdirSync(path.join(runDir, "artifacts"), { recursive: true });
  const diagnostics = [
    {
      runId,
      proofId: proofContext.proofId,
      severity: status === "manual-required" ? "warning" : "error",
      ruleId: "PROOF-MANUAL",
      message,
      file: ".",
      line: 1,
    },
  ];
  writeNdjson(path.join(runDir, "diagnostics.ndjson"), diagnostics);
  writeNdjson(path.join(runDir, "events.ndjson"), [
    { type: "proof-started", runId, proofId: proofContext.proofId, timestamp: startedAt },
    {
      type: "proof-finished",
      runId,
      proofId: proofContext.proofId,
      timestamp: startedAt,
      status,
    },
  ]);
  const proofRun = baseProofRun({
    root,
    runId,
    proofContext,
    status,
    exitCode: 1,
    startedAt,
    endedAt: startedAt,
    command: [],
    diagnosticCount: diagnostics.length,
  });
  writeProofFiles(root, runDir, proofRun);
  return { proofRun, diagnostics };
}

function writeProofRunEnvelope({
  root,
  runId,
  proofContext,
  harnessReport,
  pinned,
}) {
  const summary = harnessReport.summary;
  const runDir = proofRunDir(root, runId);
  const proofRun = baseProofRun({
    root,
    runId,
    proofContext,
    status: summary.status,
    exitCode: summary.exitCode,
    startedAt: summary.startedAt,
    endedAt: summary.endedAt,
    command: summary.command,
    diagnosticCount: summary.diagnosticCount,
    pinned,
  });
  proofRun.harness = {
    summary: `${PROOF_STORAGE_DIR}/runs/${runId}/summary.json`,
    rawStdout: summary.artifacts.stdout,
    rawStderr: summary.artifacts.stderr,
  };
  writeProofFiles(root, runDir, proofRun);
  return proofRun;
}

function writeProofFiles(root, runDir, proofRun) {
  fs.mkdirSync(runDir, { recursive: true });
  const summaryText = [
    `# Proof ${proofRun.proofId}`,
    "",
    `- Run: ${proofRun.runId}`,
    `- Status: ${proofRun.status}`,
    `- Commit: ${proofRun.git?.commit ?? "unknown"}`,
    `- Capability: ${proofRun.scope?.capability ?? "unknown"}`,
    `- Diagnostics: ${proofRun.diagnosticCount}`,
    "",
  ].join("\n");
  fs.writeFileSync(path.join(runDir, "summary.md"), summaryText, "utf8");
  fs.writeFileSync(
    path.join(runDir, "attestation.json"),
    `${JSON.stringify(attestationFor(proofRun), null, 2)}\n`,
    "utf8",
  );
  proofRun.artifacts = collectProofArtifactRecords(root, runDir);
  fs.writeFileSync(
    path.join(runDir, "proof-run.json"),
    `${JSON.stringify(proofRun, null, 2)}\n`,
    "utf8",
  );
}

function collectProofArtifactRecords(root, runDir) {
  const fixed = [
    "summary.md",
    "summary.json",
    "events.ndjson",
    "diagnostics.ndjson",
    "raw/stdout.log",
    "raw/stderr.log",
    "attestation.json",
  ];
  const discovered = [];
  const artifactRoot = path.join(runDir, "artifacts");
  if (fs.existsSync(artifactRoot)) {
    const stack = [artifactRoot];
    while (stack.length > 0) {
      const current = stack.pop();
      for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
        const fullPath = path.join(current, entry.name);
        if (entry.isDirectory()) stack.push(fullPath);
        else if (entry.isFile()) discovered.push(normalizeRel(runDir, fullPath));
      }
    }
  }
  return uniqueSorted([...fixed, ...discovered])
    .map((name) => artifactRecord(root, runDir, name))
    .filter(Boolean);
}

function artifactRecord(root, runDir, name) {
  const absolute = path.join(runDir, name);
  if (!fs.existsSync(absolute)) return null;
  const content = fs.readFileSync(absolute);
  return {
    name,
    kind: name.replace(/\..+$/u, ""),
    path: normalizeRel(root, absolute),
    sha256: crypto.createHash("sha256").update(content).digest("hex"),
    byteLength: content.byteLength,
  };
}

function attestationFor(proofRun) {
  return {
    _type: "https://in-toto.io/Statement/v1",
    subject: [
      {
        name: proofRun.proofId,
        digest: { gitCommit: proofRun.git?.commit ?? "unknown" },
      },
    ],
    predicateType: "https://ocentra.dev/attestations/proof-run/v1",
    predicate: {
      runId: proofRun.runId,
      status: proofRun.status,
      startedAt: proofRun.startedAt,
      endedAt: proofRun.endedAt,
      capability: proofRun.scope?.capability ?? null,
    },
  };
}

function updateProofManifest(root, proofRun) {
  const manifestPath = path.join(proofStorageRoot(root), PROOF_MANIFEST);
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  const current = fs.existsSync(manifestPath)
    ? JSON.parse(fs.readFileSync(manifestPath, "utf8"))
    : { schemaVersion: 1, runs: [] };
  current.runs = [
    ...current.runs.filter((run) => run.runId !== proofRun.runId),
    {
      runId: proofRun.runId,
      proofId: proofRun.proofId,
      status: proofRun.status,
      startedAt: proofRun.startedAt,
      endedAt: proofRun.endedAt,
      commit: proofRun.git?.commit ?? null,
      summaryPath: `${PROOF_STORAGE_DIR}/runs/${proofRun.runId}/proof-run.json`,
      pinned: proofRun.pinned,
    },
  ].sort((left, right) =>
    String(right.startedAt).localeCompare(String(left.startedAt)),
  );
  fs.writeFileSync(manifestPath, `${JSON.stringify(current, null, 2)}\n`, "utf8");
}

function listProofRuns(root) {
  const manifestPath = path.join(proofStorageRoot(root), PROOF_MANIFEST);
  if (!fs.existsSync(manifestPath)) return [];
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  return (manifest.runs ?? [])
    .map((entry) => readProofRun(root, entry.runId))
    .filter(Boolean);
}

function latestProofRun(root, proofId) {
  return (
    listProofRuns(root)
      .filter((run) => !proofId || run.proofId === proofId)
      .sort((left, right) =>
        String(right.startedAt).localeCompare(String(left.startedAt)),
      )[0] ?? null
  );
}

function readProofRun(root, runId) {
  const proofRunPath = path.join(proofRunDir(root, runId), "proof-run.json");
  return fs.existsSync(proofRunPath)
    ? JSON.parse(fs.readFileSync(proofRunPath, "utf8"))
    : null;
}

function readProofDiagnostics(root, runId) {
  const diagnosticsPath = path.join(proofRunDir(root, runId), "diagnostics.ndjson");
  if (!fs.existsSync(diagnosticsPath)) return [];
  return fs
    .readFileSync(diagnosticsPath, "utf8")
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function pruneProofRuns({ root }) {
  const runs = listProofRuns(root).sort((left, right) =>
    String(right.startedAt).localeCompare(String(left.startedAt)),
  );
  const keep = new Set();
  const remove = new Set();
  const byProof = new Map();
  const now = Date.now();
  for (const run of runs) {
    if (run.pinned) keep.add(run.runId);
    const proofRuns = byProof.get(run.proofId) ?? [];
    proofRuns.push(run);
    byProof.set(run.proofId, proofRuns);
    if (DEFAULT_PROOF_RETENTION.pruneAfterDays !== null) {
      const ageMs = now - Date.parse(run.startedAt);
      if (
        Number.isFinite(ageMs) &&
        ageMs > DEFAULT_PROOF_RETENTION.pruneAfterDays * 24 * 60 * 60 * 1000
      ) {
        remove.add(run.runId);
      }
    }
  }
  for (const proofRuns of byProof.values()) {
    for (const run of proofRuns.slice(0, DEFAULT_PROOF_RETENTION.maxRunsPerProof)) {
      keep.add(run.runId);
    }
    for (const run of proofRuns.slice(DEFAULT_PROOF_RETENTION.maxRunsPerProof)) {
      remove.add(run.runId);
    }
  }
  for (const run of runs.filter((entry) => entry.status !== "passed").slice(0, DEFAULT_PROOF_RETENTION.maxFailedRuns)) {
    keep.add(run.runId);
  }
  const removed = [];
  for (const run of runs) {
    if (!remove.has(run.runId) || keep.has(run.runId)) continue;
    fs.rmSync(proofRunDir(root, run.runId), { recursive: true, force: true });
    removed.push(run.runId);
  }
  rewriteProofManifest(root);
  return { ok: true, root, removed };
}

function rewriteProofManifest(root) {
  const runsRoot = path.join(proofStorageRoot(root), "runs");
  if (!fs.existsSync(runsRoot)) return;
  const runs = fs
    .readdirSync(runsRoot, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => readProofRun(root, entry.name))
    .filter(Boolean)
    .sort((left, right) =>
      String(right.startedAt).localeCompare(String(left.startedAt)),
    );
  const manifestPath = path.join(proofStorageRoot(root), PROOF_MANIFEST);
  fs.mkdirSync(path.dirname(manifestPath), { recursive: true });
  fs.writeFileSync(
    manifestPath,
    `${JSON.stringify(
      {
        schemaVersion: 1,
        runs: runs.map((run) => ({
          runId: run.runId,
          proofId: run.proofId,
          status: run.status,
          startedAt: run.startedAt,
          endedAt: run.endedAt,
          commit: run.git?.commit ?? null,
          summaryPath: `${PROOF_STORAGE_DIR}/runs/${run.runId}/proof-run.json`,
          pinned: run.pinned,
        })),
      },
      null,
      2,
    )}\n`,
    "utf8",
  );
}

export {
  PROOF_STORAGE_DIR,
  PROOF_MANIFEST,
  DEFAULT_PROOF_RETENTION,
  proofHarnessConfig,
  proofStorageRoot,
  proofRunDir,
  createProofRunId,
  sanitizeRelativeArtifactName,
  redactSecrets,
  redactedJson,
  gitState,
  writeNdjson,
  baseProofRun,
  writeManualProofRun,
  writeProofRunEnvelope,
  writeProofFiles,
  updateProofManifest,
  listProofRuns,
  latestProofRun,
  readProofRun,
  readProofDiagnostics,
  pruneProofRuns,
};
