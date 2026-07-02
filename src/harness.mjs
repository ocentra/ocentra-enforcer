import fs from 'node:fs';
import path from 'node:path';
import process from 'node:process';
import { spawnSync } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { normalizeRel } from './path-utils.mjs';
import {
  dedupeDiagnostics,
  parseDiagnostics,
  parseJsonLines,
  parseJsonPayload,
  parsePytestText,
  parseTscText,
  parserDiagnostic,
  rustMessageToDiagnostic,
  sarifSeverity,
  sortDiagnostics,
} from './harness-parsers.mjs';

const DEFAULT_HARNESS_CONFIG = Object.freeze({
  storageDir: '.enforce',
  store: 'ndjson-duckdb',
  maxArtifactBytes: 8000,
  maxRuns: 50,
  maxRunsPerTool: 20,
  maxFailedRuns: 20,
  pruneAfterDays: 14,
});

const LEGACY_STORAGE_DIR = '.ocentra-enforcer';
const SECRET_REDACTION_PATTERNS = [
  /\bgh[pousr]_[A-Za-z0-9_]{20,}\b/gu,
  /\bAKIA[0-9A-Z]{16}\b/gu,
  /\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b/gu,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/gu,
  /\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{16,}["']/giu,
  /-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----/gu,
];

export function runHarness(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const command = args.command ?? [];
  if (!Array.isArray(command) || command.length === 0) throw new Error('run requires command: [executable, ...args]');
  const harnessConfig = normalizeHarnessConfig(args.harness);
  const runId = args.runId ?? createRunId();
  const startedAt = new Date().toISOString();
  const storageRoot = harnessStorageRoot(root, harnessConfig);
  const runDir = path.join(storageRoot, 'runs', runId);
  fs.mkdirSync(path.join(runDir, 'raw'), { recursive: true });
  fs.mkdirSync(path.join(storageRoot, 'db'), { recursive: true });
  const cwd = args.cwd ? path.resolve(root, args.cwd) : root;
  const tool = args.tool ?? command[0];
  const language = args.language ?? inferLanguage(tool);

  const result = spawnSync(command[0], command.slice(1), {
    cwd,
    encoding: 'utf8',
    shell: false,
    env: { ...process.env, ...(args.env ?? {}) },
  });
  const endedAt = new Date().toISOString();
  const stdout = redactSecrets(result.stdout ?? '');
  const stderr = redactSecrets(result.stderr ?? '');
  fs.writeFileSync(path.join(runDir, 'raw', 'stdout.log'), stdout, 'utf8');
  fs.writeFileSync(path.join(runDir, 'raw', 'stderr.log'), stderr, 'utf8');

  const exitCode = result.status ?? (result.error ? 1 : 0);
  const diagnostics = sortDiagnostics(dedupeDiagnostics([
    ...parseDiagnostics({ root, runId, tool: args.tool ?? command[0], stdout, stderr }),
    ...(exitCode === 0
      ? []
      : [
          {
            runId,
            tool,
            language,
            severity: 'error',
            ruleId: 'HAR-1.1',
            file: '.',
            line: 1,
            message: `Command failed with exit code ${exitCode}.`,
            source: stderr.trim().split(/\r?\n/u).slice(0, 8).join('\n') || stdout.trim().split(/\r?\n/u).slice(0, 8).join('\n') || null,
          },
        ]),
  ]));
  const summary = {
    runId,
    root,
    profile: args.profile ?? 'strict',
    tool,
    language,
    cwd: normalizeRel(root, cwd) || '.',
    crateName: args.crateName ?? null,
    packageName: args.packageName ?? null,
    domain: args.domain ?? null,
    tags: normalizeTags(args.tags),
    command,
    pinned: Boolean(args.pin ?? args.pinned),
    status: exitCode === 0 ? 'passed' : 'failed',
    exitCode,
    startedAt,
    endedAt,
    diagnosticCount: diagnostics.length,
    bySeverity: countBy(diagnostics, 'severity'),
    artifacts: {
      stdout: normalizeRel(root, path.join(runDir, 'raw', 'stdout.log')),
      stderr: normalizeRel(root, path.join(runDir, 'raw', 'stderr.log')),
      diagnostics: normalizeRel(root, path.join(runDir, 'diagnostics.ndjson')),
      events: normalizeRel(root, path.join(runDir, 'events.ndjson')),
    },
    storage: {
      root: normalizeRel(root, storageRoot),
      retention: retentionSummary(harnessConfig),
    },
    duckdb: writeDuckDbStatus(root, storageRoot),
  };
  writeNdjson(path.join(runDir, 'diagnostics.ndjson'), diagnostics);
  writeNdjson(path.join(runDir, 'events.ndjson'), [
    { type: 'run-started', runId, timestamp: startedAt, tool: summary.tool, command },
    { type: 'run-finished', runId, timestamp: endedAt, status: summary.status, exitCode, diagnosticCount: diagnostics.length },
  ]);
  const summaryPath = path.join(runDir, 'summary.json');
  fs.writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
  const prune = pruneRuns({ root, harness: harnessConfig });
  summary.pruned = prune.removed;
  if (fs.existsSync(runDir)) fs.writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
  writeManifest(root, storageRoot, runId, summary);
  return { ok: exitCode === 0, summary, diagnostics };
}

export function listRuns(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  return allRuns(root, args.harness)
    .filter((run) => matchesRunQuery(run, args))
    .filter(Boolean)
    .sort((a, b) => String(b.startedAt).localeCompare(String(a.startedAt)))
    .slice(0, args.limit ?? 20);
}

export function runSummary(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  if (args.runId) return readSummary(root, args.runId, args.harness);
  return listRuns({ ...args, root, limit: 1 })[0] ?? null;
}

export function runDiagnostics(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const run = runSummary(args);
  if (!run) return { ok: false, diagnostics: [], message: 'No harness run found.' };
  const diagnostics = readNdjson(path.join(root, run.artifacts?.diagnostics ?? path.join(storageDirFromSummary(run), 'runs', run.runId, 'diagnostics.ndjson')));
  const filtered = diagnostics
    .filter((diagnostic) => !args.severity || diagnostic.severity === args.severity)
    .filter((diagnostic) => !args.file || diagnostic.file === args.file)
    .slice(0, args.limit ?? 50);
  return { ok: true, runId: run.runId, diagnostics: filtered };
}

export function lastFailure(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const failedRun = listRuns({ ...args, root, limit: args.limit ?? 50 }).find((run) => run.status === 'failed');
  if (!failedRun) return { ok: true, found: false, message: 'No failed harness run found.' };
  const diagnostics = runDiagnostics({ ...args, root, runId: failedRun.runId, limit: args.diagnosticLimit ?? 10 }).diagnostics;
  return { ok: true, found: true, run: failedRun, diagnostics };
}

export function readArtifact(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const harnessConfig = normalizeHarnessConfig(args.harness);
  const run = runSummary(args);
  if (!run) return { ok: false, text: '', message: 'No harness run found.' };
  const artifact = args.artifact ?? 'stderr';
  const artifactPath = run.artifacts?.[artifact];
  if (!artifactPath) return { ok: false, text: '', message: `Unknown artifact: ${artifact}` };
  const absolute = path.resolve(root, artifactPath);
  if (!isInsideRoot(root, absolute)) {
    return { ok: false, text: '', message: `Artifact path escapes harness root: ${artifactPath}` };
  }
  const text = fs.existsSync(absolute) ? fs.readFileSync(absolute, 'utf8') : '';
  return {
    ok: true,
    runId: run.runId,
    artifact,
    path: artifactPath,
    text: redactSecrets(text).slice(0, args.limitBytes ?? harnessConfig.maxArtifactBytes),
  };
}

export function resetRuns(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const removed = [];
  for (const storeRoot of candidateStorageRoots(root, args.harness)) {
    if (fs.existsSync(storeRoot)) {
      fs.rmSync(storeRoot, { recursive: true, force: true });
      removed.push(normalizeRel(root, storeRoot));
    }
  }
  return { ok: true, root, removed };
}

export function pruneRuns(args = {}) {
  const root = path.resolve(args.root ?? process.cwd());
  const harnessConfig = normalizeHarnessConfig(args.harness);
  const runs = allRuns(root, harnessConfig);
  const keep = new Set();
  const remove = new Set();
  const now = Date.now();

  const sorted = runs.sort((a, b) => String(b.startedAt).localeCompare(String(a.startedAt)));
  for (const [index, run] of sorted.entries()) {
    if (harnessConfig.maxRuns !== null && index >= harnessConfig.maxRuns) remove.add(run.runId);
    if (harnessConfig.pruneAfterDays !== null) {
      const ageMs = now - Date.parse(run.startedAt);
      if (Number.isFinite(ageMs) && ageMs > harnessConfig.pruneAfterDays * 24 * 60 * 60 * 1000) remove.add(run.runId);
    }
  }

  for (const run of sorted.filter((entry) => entry.pinned === true)) keep.add(run.runId);
  for (const run of sorted.filter((entry) => entry.status === 'failed').slice(0, harnessConfig.maxFailedRuns ?? sorted.length)) keep.add(run.runId);
  const byTool = new Map();
  for (const run of sorted) {
    const toolRuns = byTool.get(run.tool) ?? [];
    toolRuns.push(run);
    byTool.set(run.tool, toolRuns);
  }
  for (const toolRuns of byTool.values()) {
    for (const run of toolRuns.slice(0, harnessConfig.maxRunsPerTool ?? toolRuns.length)) keep.add(run.runId);
  }

  const removed = [];
  for (const run of sorted) {
    if (!remove.has(run.runId) || keep.has(run.runId)) continue;
    const runDir = path.join(root, storageDirFromSummary(run), 'runs', run.runId);
    if (fs.existsSync(runDir)) {
      fs.rmSync(runDir, { recursive: true, force: true });
      removed.push(run.runId);
    }
  }
  rewriteManifest(root, harnessStorageRoot(root, harnessConfig));
  return { ok: true, root, removed };
}

function writeNdjson(filePath, rows) {
  fs.writeFileSync(filePath, rows.map((row) => JSON.stringify(row)).join('\n') + (rows.length > 0 ? '\n' : ''), 'utf8');
}

function readNdjson(filePath) {
  if (!fs.existsSync(filePath)) return [];
  return fs
    .readFileSync(filePath, 'utf8')
    .split(/\r?\n/u)
    .filter(Boolean)
    .map((line) => JSON.parse(line));
}

function normalizeHarnessConfig(value = {}) {
  const config = { ...DEFAULT_HARNESS_CONFIG, ...(value ?? {}) };
  return {
    ...config,
    storageDir: sanitizeStorageDir(config.storageDir),
    maxRuns: nullableNumber(config.maxRuns),
    maxRunsPerTool: nullableNumber(config.maxRunsPerTool),
    maxFailedRuns: nullableNumber(config.maxFailedRuns),
    pruneAfterDays: nullableNumber(config.pruneAfterDays),
  };
}

function redactSecrets(value) {
  let text = String(value ?? '');
  for (const pattern of SECRET_REDACTION_PATTERNS) text = text.replace(pattern, '[REDACTED]');
  return text;
}

function isInsideRoot(root, candidate) {
  const relative = path.relative(path.resolve(root), path.resolve(candidate));
  return relative === '' || (!relative.startsWith('..') && !path.isAbsolute(relative));
}

function sanitizeStorageDir(value) {
  const storageDir = value || DEFAULT_HARNESS_CONFIG.storageDir;
  if (path.isAbsolute(storageDir) || storageDir.includes('..')) throw new Error(`Invalid harness storageDir: ${storageDir}`);
  return storageDir.replace(/\\/gu, '/');
}

function nullableNumber(value) {
  return value === undefined ? null : value;
}

function harnessStorageRoot(root, harnessConfig) {
  return path.join(root, harnessConfig.storageDir);
}

function candidateStorageRoots(root, harness) {
  const harnessConfig = normalizeHarnessConfig(harness);
  return [...new Set([harnessStorageRoot(root, harnessConfig), path.join(root, LEGACY_STORAGE_DIR)])];
}

function allRuns(root, harness) {
  const runs = [];
  const seen = new Set();
  for (const storageRoot of candidateStorageRoots(root, harness)) {
    const runsRoot = path.join(storageRoot, 'runs');
    if (!fs.existsSync(runsRoot)) continue;
    for (const entry of fs.readdirSync(runsRoot, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      const summary = readSummaryFromStorage(root, storageRoot, entry.name);
      if (!summary || seen.has(summary.runId)) continue;
      seen.add(summary.runId);
      runs.push(summary);
    }
  }
  return runs;
}

function readSummary(root, runId, harness) {
  for (const storageRoot of candidateStorageRoots(root, harness)) {
    const summary = readSummaryFromStorage(root, storageRoot, runId);
    if (summary) return summary;
  }
  return null;
}

function readSummaryFromStorage(root, storageRoot, runId) {
  const summaryPath = path.join(storageRoot, 'runs', runId, 'summary.json');
  if (!fs.existsSync(summaryPath)) return null;
  const summary = JSON.parse(fs.readFileSync(summaryPath, 'utf8'));
  return {
    ...summary,
    storage: summary.storage ?? {
      root: normalizeRel(root, storageRoot),
      retention: retentionSummary(DEFAULT_HARNESS_CONFIG),
    },
  };
}

function writeManifest(root, storageRoot, runId, summary) {
  const manifestPath = path.join(storageRoot, 'db', 'ingest-manifest.json');
  const current = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, 'utf8')) : { runs: [] };
  current.runs = [
    ...current.runs.filter((entry) => entry.runId !== runId),
    {
      runId,
      summaryPath: `${normalizeRel(root, storageRoot)}/runs/${runId}/summary.json`,
      ingestedAt: new Date().toISOString(),
      tool: summary.tool,
      status: summary.status,
      crateName: summary.crateName,
      packageName: summary.packageName,
      domain: summary.domain,
      tags: summary.tags,
      duckdb: summary.duckdb,
    },
  ];
  fs.writeFileSync(manifestPath, `${JSON.stringify(current, null, 2)}\n`, 'utf8');
}

function rewriteManifest(root, storageRoot) {
  if (!fs.existsSync(storageRoot)) return;
  fs.mkdirSync(path.join(storageRoot, 'db'), { recursive: true });
  const currentRuns = allRuns(root, { storageDir: normalizeRel(root, storageRoot) }).filter((run) => storageDirFromSummary(run) === normalizeRel(root, storageRoot));
  const manifest = {
    runs: currentRuns.map((run) => ({
      runId: run.runId,
      summaryPath: `${storageDirFromSummary(run)}/runs/${run.runId}/summary.json`,
      ingestedAt: new Date().toISOString(),
      tool: run.tool,
      status: run.status,
      crateName: run.crateName,
      packageName: run.packageName,
      domain: run.domain,
      tags: run.tags ?? [],
      duckdb: run.duckdb,
    })),
  };
  fs.writeFileSync(path.join(storageRoot, 'db', 'ingest-manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`, 'utf8');
}

function writeDuckDbStatus(root, storageRoot) {
  const status = {
    mode: 'optional',
    available: false,
    database: `${normalizeRel(root, storageRoot)}/db/harness.duckdb`,
    detail: 'DuckDB ingestion is reserved; NDJSON is authoritative when duckdb is not installed.',
  };
  fs.writeFileSync(path.join(storageRoot, 'db', 'duckdb-status.json'), `${JSON.stringify(status, null, 2)}\n`, 'utf8');
  return status;
}

function storageDirFromSummary(summary) {
  return summary.storage?.root ?? LEGACY_STORAGE_DIR;
}

function matchesRunQuery(run, args) {
  if (args.runId && run.runId !== args.runId) return false;
  if (args.status && run.status !== args.status) return false;
  if (args.tool && run.tool !== args.tool) return false;
  if (args.crateName && run.crateName !== args.crateName) return false;
  if (args.packageName && run.packageName !== args.packageName) return false;
  if (args.domain && run.domain !== args.domain) return false;
  if (args.tag && !(run.tags ?? []).includes(args.tag)) return false;
  return true;
}

function normalizeTags(tags = []) {
  return Array.isArray(tags) ? [...new Set(tags.map((tag) => String(tag).trim()).filter(Boolean))] : [];
}

function retentionSummary(config) {
  return {
    maxRuns: config.maxRuns,
    maxRunsPerTool: config.maxRunsPerTool,
    maxFailedRuns: config.maxFailedRuns,
    pruneAfterDays: config.pruneAfterDays,
  };
}

function countBy(rows, key) {
  const counts = {};
  for (const row of rows) counts[row[key] ?? 'unknown'] = (counts[row[key] ?? 'unknown'] ?? 0) + 1;
  return counts;
}

function createRunId() {
  return `${new Date().toISOString().replace(/[-:.TZ]/gu, '').slice(0, 14)}-${randomUUID().slice(0, 8)}`;
}

function inferLanguage(tool) {
  if (/cargo|rust|clippy/u.test(tool)) return 'rust';
  if (/eslint|tsc|vitest|jest|npm|pnpm|yarn/u.test(tool)) return 'typescript';
  if (/pytest|ruff|pyright|mypy|python/u.test(tool)) return 'python';
  return 'common';
}
