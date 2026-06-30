import crypto from "node:crypto";
import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

import {
  decodeProofClaimArguments,
  decodeProofQueryArguments,
  decodeProofRegistry,
  decodeProofRouteRequest,
  decodeProofRunArguments,
} from "../schemas/effect/enforcer-schemas.mjs";
import { runHarness } from "./harness.mjs";
import { normalizeRel, repoAbsolute, toPosix, uniqueSorted } from "./path-utils.mjs";

const DEFAULT_PACK_ROOT = path.resolve(path.join(path.dirname(fileURLToPath(import.meta.url)), ".."));
const PROOF_STORAGE_DIR = ".enforce/proofs";
const PROOF_MANIFEST = "db/proof-manifest.json";
const DEFAULT_PROOF_RETENTION = Object.freeze({
  maxRunsPerProof: 20,
  maxFailedRuns: 20,
  maxArtifactBytes: 50 * 1024 * 1024,
  pruneAfterDays: 14,
  pinPrReadyDays: 30,
});

const SCRIPT_REFERENCE_PATTERNS = [
  "test-results",
  "output/",
  "docs/proof",
  "docs/expectations",
  "docs/plans",
  "packages/schema-domain",
  "packages/agent-protocol-domain",
  "crates/agent-protocol",
  "crates/agent-service",
  "apps/portal",
];

export function loadProofRegistry(packRoot = DEFAULT_PACK_ROOT) {
  const registryPath = path.join(packRoot, "proof", "proofs.json");
  return decodeProofRegistry(JSON.parse(fs.readFileSync(registryPath, "utf8")));
}

export function routeProofs(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRouteRequest(input);
  const registry = loadProofRegistry(packRoot);
  const root = path.resolve(args.root ?? process.cwd());
  const profileName = args.profile ?? "strict";
  const explicitProofId = args.proofId ?? null;
  const familyKeys = explicitProofId ? new Set() : proofFamilyKeys(args);
  const definitions = explicitProofId
    ? registry.proofs.filter((proof) => proof.id === explicitProofId)
    : registry.proofs.filter((proof) => proofMatchesRoute(proof, args, familyKeys));

  return {
    ok: true,
    productName: registry.productName,
    profileName,
    index: "proof/INDEX.md",
    scope: describeProofRouteScope(args),
    docs: uniqueSorted(definitions.flatMap((proof) => proof.docs ?? [])),
    proofs: definitions.map(compactProofDefinition),
  };
}

export function inventoryProofs(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const scriptsRoot = path.join(root, "scripts", "test");
  const scripts = fs.existsSync(scriptsRoot)
    ? collectScriptFiles(scriptsRoot).map((filePath) => classifyProofScript(root, filePath))
    : [];
  const totals = {
    scripts: scripts.length,
    proofNamed: scripts.filter((script) => script.name.includes("proof")).length,
    spawnCommands: scripts.filter((script) => script.signals.spawn).length,
    writesProof: scripts.filter((script) => script.signals.writesProof).length,
    readsProof: scripts.filter((script) => script.signals.readsProof).length,
    manualOrDevice: scripts.filter((script) => script.signals.manualOrDevice).length,
    importsBuiltOrSchemaParse: scripts.filter((script) => script.signals.importsBuiltOrSchemaParse).length,
  };
  const includeScripts = Boolean(args.includeScripts);
  const scriptLimit = Number.isFinite(args.limit) ? Math.max(0, args.limit) : 20;
  const selectedScripts = includeScripts ? scripts.slice(0, scriptLimit) : [];
  return {
    ok: true,
    root,
    scriptsRoot: normalizeRel(root, scriptsRoot),
    totals,
    byFamily: countBy(scripts, "family"),
    byCapability: countCapabilities(scripts),
    references: countReferences(scripts),
    scriptRowsIncluded: includeScripts,
    scriptLimit: includeScripts ? scriptLimit : 0,
    omittedScriptCount: Math.max(0, scripts.length - selectedScripts.length),
    scripts: selectedScripts,
  };
}

export function runProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRunArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadProofRegistry(packRoot);
  const proofId = args.proofId ?? "ad-hoc-command-proof";
  const definition =
    registry.proofs.find((proof) => proof.id === proofId) ??
    adHocProofDefinition(proofId, args.command?.length ? "command" : "manual-artifact");
  const command = args.command?.length ? args.command : definition.commands?.[0] ?? [];
  const capability = args.capability ?? definition.capabilities?.[0] ?? "local";
  const runId = args.runId ?? createProofRunId(proofId);
  const git = gitState(root);
  const proofContext = {
    schemaVersion: 1,
    proofId,
    title: definition.title,
    family: definition.family,
    collector: definition.collector,
    profile: args.profile ?? "strict",
    root,
    scope: {
      files: args.files ?? [],
      plan: args.plan ?? null,
      capability,
    },
    git,
    claimsProved: definition.claimsProved ?? [],
    claimsNotProved: definition.claimsNotProved ?? [],
  };

  if (command.length === 0) {
    const manual = writeManualProofRun({
      root,
      runId,
      proofContext,
      status: capability === "manual-required" || definition.collector === "manual-artifact" ? "manual-required" : "unavailable",
      message: "No executable command was provided; proof requires external/manual evidence.",
    });
    updateProofManifest(root, manual.proofRun);
    pruneProofRuns({ root });
    return { ok: false, proofRun: manual.proofRun, diagnostics: manual.diagnostics };
  }

  const harnessReport = runHarness({
    root,
    profile: args.profile ?? "strict",
    tool: `proof:${proofId}`,
    language: "common",
    harness: proofHarnessConfig(),
    command,
    runId,
    packageName: args.plan ?? null,
    domain: proofId,
    tags: uniqueSorted(["proof", proofId, ...(args.tags ?? [])]),
  });
  const proofRun = writeProofRunEnvelope({
    root,
    runId,
    proofContext,
    harnessReport,
    pinned: Boolean(args.pin),
  });
  updateProofManifest(root, proofRun);
  pruneProofRuns({ root });
  return { ok: proofRun.status === "passed", proofRun, diagnostics: harnessReport.diagnostics };
}

export function proofStatus(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const runs = listProofRuns(root)
    .filter((run) => !args.proofId || run.proofId === args.proofId)
    .filter((run) => !args.status || run.status === args.status)
    .sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)))
    .slice(0, args.limit ?? 20);
  return { ok: true, root, runs };
}

export function proofLastFailure(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = listProofRuns(root)
    .filter((entry) => !args.proofId || entry.proofId === args.proofId)
    .sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)))
    .find((entry) => entry.status === "failed" || entry.status === "manual-required" || entry.status === "unavailable");
  if (!run) return { ok: true, found: false, message: "No failed proof run found." };
  return {
    ok: true,
    found: true,
    proofRun: run,
    diagnostics: readProofDiagnostics(root, run.runId).slice(0, args.diagnosticLimit ?? args.limit ?? 10),
  };
}

export function proofDiagnostics(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = args.runId ? readProofRun(root, args.runId) : latestProofRun(root, args.proofId);
  if (!run) return { ok: false, diagnostics: [], message: "No proof run found." };
  const diagnostics = readProofDiagnostics(root, run.runId).slice(0, args.limit ?? 50);
  return { ok: true, runId: run.runId, proofId: run.proofId, diagnostics };
}

export function proofArtifact(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = args.runId ? readProofRun(root, args.runId) : latestProofRun(root, args.proofId);
  if (!run) return { ok: false, text: "", message: "No proof run found." };
  const artifact = args.artifact ?? "summary";
  const artifactRecord = run.artifacts.find((entry) => entry.kind === artifact || entry.name === artifact);
  if (!artifactRecord) return { ok: false, text: "", message: `Unknown proof artifact: ${artifact}` };
  const absolute = path.join(root, artifactRecord.path);
  const text = fs.existsSync(absolute) ? fs.readFileSync(absolute, "utf8") : "";
  return {
    ok: true,
    runId: run.runId,
    proofId: run.proofId,
    artifact,
    path: artifactRecord.path,
    text: text.slice(0, args.limitBytes ?? 8000),
  };
}

export function proofReset(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const storage = proofStorageRoot(root);
  if (fs.existsSync(storage)) fs.rmSync(storage, { recursive: true, force: true });
  return { ok: true, root, removed: [PROOF_STORAGE_DIR] };
}

export function proofPrune(input = {}) {
  const args = decodeProofQueryArguments(input);
  return pruneProofRuns({ root: path.resolve(args.root ?? process.cwd()) });
}

export function proofExport(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const runs = listProofRuns(root)
    .filter((run) => !args.proofId || run.proofId === args.proofId)
    .slice(0, args.limit ?? 20);
  return {
    ok: true,
    root,
    bundle: {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      runs,
      note: "This is a manifest-only export. CI should upload artifacts separately instead of committing proof outputs.",
    },
  };
}

export function claimProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofClaimArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadProofRegistry(packRoot);
  const proofIds = uniqueSorted(args.proofIds?.length ? args.proofIds : args.proofId ? [args.proofId] : []);
  const ids = proofIds.length > 0 ? proofIds : registry.proofs.filter((proof) => proof.requiredForPrReady).map((proof) => proof.id);
  const currentGit = gitState(root);
  const violations = [];
  const accepted = [];

  for (const proofId of ids) {
    const definition = registry.proofs.find((proof) => proof.id === proofId) ?? null;
    const run = latestProofRun(root, proofId);
    if (!run) {
      violations.push(claimViolation(proofId, "missing-proof-run", "No proof run exists for this proof id."));
      continue;
    }
    if (run.status !== "passed") {
      violations.push(claimViolation(proofId, "proof-not-passed", `Latest proof status is ${run.status}.`));
    }
    if (currentGit.commit && run.git?.commit && currentGit.commit !== run.git.commit) {
      violations.push(
        claimViolation(proofId, "stale-commit", `Proof commit ${run.git.commit} does not match current commit ${currentGit.commit}.`),
      );
    }
    if (args.prReady && currentGit.dirty && !args.allowDirty) {
      violations.push(claimViolation(proofId, "dirty-worktree", "PR-ready proof claims require a clean worktree unless allowDirty is explicit."));
    }
    for (const artifact of run.artifacts ?? []) {
      if (!fs.existsSync(path.join(root, artifact.path))) {
        violations.push(claimViolation(proofId, "missing-artifact", `Missing artifact ${artifact.path}.`));
      }
    }
    for (const requiredPath of definition?.requiredPaths ?? []) {
      if (!fs.existsSync(repoAbsolute(root, requiredPath))) {
        violations.push(claimViolation(proofId, "deleted-required-path", `Required path is missing: ${requiredPath}.`));
      }
    }
    accepted.push({ proofId, runId: run.runId, status: run.status, commit: run.git?.commit ?? null });
  }

  return {
    ok: violations.length === 0,
    root,
    claim: {
      schemaVersion: 1,
      claimId: args.claimId ?? createProofRunId("claim"),
      prReady: Boolean(args.prReady),
      proofIds: ids,
      checkedAt: new Date().toISOString(),
      currentGit,
      accepted,
      violations,
    },
  };
}

export function runProofCli(tokens, options = {}) {
  const parsed = stripNullish(parseProofCli(tokens, options.defaultRoot ?? process.cwd()));
  const packRoot = options.packRoot ?? DEFAULT_PACK_ROOT;
  let result;
  switch (parsed.cliCommand) {
    case "route":
      result = routeProofs(parsed, packRoot);
      break;
    case "run":
      result = runProof(parsed, packRoot);
      break;
    case "status":
      result = proofStatus(parsed);
      break;
    case "inventory":
      result = inventoryProofs(parsed);
      break;
    case "claim":
      result = claimProof(parsed, packRoot);
      break;
    case "last-failure":
      result = proofLastFailure(parsed);
      break;
    case "diagnostics":
      result = proofDiagnostics(parsed);
      break;
    case "artifact":
      result = proofArtifact(parsed);
      break;
    case "reset":
      result = proofReset(parsed);
      break;
    case "prune":
      result = proofPrune(parsed);
      break;
    case "export":
      result = proofExport(parsed);
      break;
    default:
      throw new Error(`Unknown proof command: ${parsed.cliCommand}`);
  }
  return {
    command: parsed.cliCommand,
    json: parsed.json,
    result,
    exitCode: result?.ok === false ? 1 : 0,
    text: formatProofReport(parsed.cliCommand, result),
  };
}

export function formatProofReport(command, result) {
  if (command === "route") {
    return `Proof route: ${result.proofs.length} proof(s), docs=${result.docs.length}`;
  }
  if (command === "run") {
    return `Proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  }
  if (command === "status") {
    return result.runs.map((run) => `${run.runId} ${run.status} ${run.proofId}`).join("\n");
  }
  if (command === "artifact") return result.text ?? result.message ?? "";
  if (command === "diagnostics") {
    return (result.diagnostics ?? []).map((diagnostic) => `${diagnostic.ruleId}: ${diagnostic.message}`).join("\n");
  }
  return JSON.stringify(result, null, 2);
}

function parseProofCli(tokens, defaultRoot) {
  const cliCommand = tokens[0] && !tokens[0].startsWith("-") ? tokens.shift() : "route";
  const parsed = {
    cliCommand,
    root: defaultRoot,
    profile: "strict",
    proofId: null,
    proofIds: [],
    files: [],
    plan: null,
    capability: null,
    commandArgs: [],
    runId: null,
    artifact: null,
    limit: null,
    limitBytes: null,
    diagnosticLimit: null,
    includeScripts: false,
    status: null,
    tags: [],
    json: false,
    pin: false,
    prReady: false,
    allowDirty: false,
    claimId: null,
  };

  for (let index = 0; index < tokens.length; index += 1) {
    const token = tokens[index];
    if (token === "--") {
      parsed.commandArgs = tokens.slice(index + 1);
      break;
    }
    if (token === "--root") parsed.root = tokens[++index] ?? parsed.root;
    else if (token === "--profile") parsed.profile = tokens[++index] ?? parsed.profile;
    else if (token === "--proof" || token === "--proof-id") parsed.proofId = tokens[++index] ?? null;
    else if (token === "--proofs") parsed.proofIds = splitList(tokens[++index] ?? "");
    else if (token === "--plan") parsed.plan = tokens[++index] ?? null;
    else if (token === "--capability") parsed.capability = tokens[++index] ?? null;
    else if (token === "--run-id") parsed.runId = tokens[++index] ?? null;
    else if (token === "--artifact") parsed.artifact = tokens[++index] ?? null;
    else if (token === "--limit") parsed.limit = Number(tokens[++index]);
    else if (token === "--limit-bytes") parsed.limitBytes = Number(tokens[++index]);
    else if (token === "--diagnostic-limit") parsed.diagnosticLimit = Number(tokens[++index]);
    else if (token === "--include-scripts") parsed.includeScripts = true;
    else if (token === "--status") parsed.status = tokens[++index] ?? null;
    else if (token === "--tag") parsed.tags.push(tokens[++index] ?? "");
    else if (token === "--claim-id") parsed.claimId = tokens[++index] ?? null;
    else if (token === "--json") parsed.json = true;
    else if (token === "--pin") parsed.pin = true;
    else if (token === "--pr-ready") parsed.prReady = true;
    else if (token === "--allow-dirty") parsed.allowDirty = true;
    else if (token === "--files") {
      for (let fileIndex = index + 1; fileIndex < tokens.length; fileIndex += 1) {
        if (tokens[fileIndex].startsWith("-")) {
          index = fileIndex - 1;
          break;
        }
        parsed.files.push(tokens[fileIndex]);
        index = fileIndex;
      }
    } else if (token.startsWith("-")) {
      throw new Error(`Unknown proof argument: ${token}`);
    }
  }

  return {
    ...parsed,
    cliCommand: parsed.cliCommand ?? "route",
    scope: parsed.files.length > 0 ? "files" : "workspace",
    command: parsed.commandArgs,
    tags: parsed.tags.filter(Boolean),
  };
}

function proofFamilyKeys(args = {}) {
  const families = new Set();
  if (args.plan) families.add(`plan:${args.plan}`);
  if (args.capability) families.add(`capability:${args.capability}`);
  if (args.scope === "workspace") families.add("scope:workspace");
  for (const file of args.files ?? []) {
    for (const family of proofFamilyKeysForFile(file)) families.add(family);
  }
  return families;
}

function proofFamilyKeysForFile(file) {
  const rel = toPosix(file);
  const lower = rel.toLowerCase();
  const families = [];
  if (lower.endsWith(".rs") || lower.includes("cargo.toml")) families.push("language:rust");
  if (/\.[cm]?[jt]sx?$/u.test(lower) || lower.endsWith("package.json")) families.push("language:typescript");
  if (lower.endsWith(".py") || lower.endsWith("pyproject.toml")) families.push("language:python");
  if (/(?:^|\/)(?:test|tests|__tests__)\/|(?:\.test|\.spec)\./u.test(lower)) families.push("kind:test");
  if (lower.startsWith("scripts/test/") || lower.includes("proof")) families.push("kind:proof-script");
  if (lower.includes("android")) families.push("capability:android-device");
  if (lower.includes("ios") || lower.includes("xcode")) families.push("capability:ios-simulator");
  return families;
}

function proofMatchesRoute(proof, args, familyKeys) {
  if (args.capability && !(proof.capabilities ?? []).includes(args.capability)) return false;
  if (args.plan && !stringListMatches([...(proof.appliesTo ?? []), ...(proof.triggers ?? [])], args.plan)) return false;
  if (familyKeys.size === 0) return false;
  const proofKeys = new Set([
    `family:${proof.family}`,
    ...(proof.languages ?? []).map((language) => `language:${language}`),
    ...(proof.capabilities ?? []).map((capability) => `capability:${capability}`),
    ...(proof.triggers ?? []),
    ...(proof.appliesTo ?? []),
  ]);
  if (familyKeys.has("scope:workspace")) return proof.appliesTo?.includes("workspace") || proof.family === "claim-integrity";
  for (const key of familyKeys) {
    if (proofKeys.has(key)) return true;
  }
  return false;
}

function compactProofDefinition(proof) {
  return {
    id: proof.id,
    title: proof.title,
    family: proof.family,
    severity: proof.severity,
    collector: proof.collector,
    capabilities: proof.capabilities,
    docs: proof.docs,
  };
}

function describeProofRouteScope(args) {
  if (args.proofId) return { mode: "proof", proofId: args.proofId };
  if (args.files?.length) return { mode: "files", files: args.files };
  if (args.plan) return { mode: "plan", plan: args.plan };
  if (args.capability) return { mode: "capability", capability: args.capability };
  return { mode: args.scope ?? "workspace" };
}

function collectScriptFiles(root) {
  const files = [];
  const stack = [root];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const fullPath = path.join(current, entry.name);
      if (entry.isDirectory()) stack.push(fullPath);
      else if (entry.isFile() && entry.name.endsWith(".mjs")) files.push(fullPath);
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

function classifyProofScript(root, filePath) {
  const text = fs.readFileSync(filePath, "utf8");
  const rel = normalizeRel(root, filePath);
  const name = path.basename(filePath).toLowerCase();
  const capabilities = scriptCapabilities(name, text);
  const references = SCRIPT_REFERENCE_PATTERNS.filter((pattern) => text.includes(pattern));
  return {
    path: rel,
    name,
    family: scriptFamily(name, text),
    capabilities,
    signals: {
      spawn: /\bspawn(?:Sync)?\b|\bexec(?:File|Sync)?\b/u.test(text),
      writesProof: /\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text),
      readsProof: /readJson|readFile\(.+proof|test-results/u.test(text),
      manualOrDevice: /manual-required|physical|ANDROID_SERIAL|\badb\b|ios|simulator|device/iu.test(text),
      importsBuiltOrSchemaParse: /dist\/|await import|Schema\.parse|\.parse\(/u.test(text),
    },
    references,
    outputRoots: uniqueSorted([...text.matchAll(/(?:test-results|output|docs\/proof)[^'"`\s,)]+/gu)].map((match) => match[0])),
  };
}

function scriptFamily(name, text) {
  const lower = `${name}\n${text}`.toLowerCase();
  if (/android|ios|device|physical|simulator|xcode|adb/u.test(lower)) return "device-manual";
  if (/junit|pytest|vitest|jest|playwright|test-results/u.test(lower)) return "test-report";
  if (/sarif|codeql|security|secret|audit/u.test(lower)) return "security-report";
  if (/parity|contract|boundary|schema/u.test(lower)) return "contract-parity";
  if (/event|network|lan|message|codec/u.test(lower)) return "event-network";
  if (/logging|tracking|custody/u.test(lower)) return "logging-custody";
  if (/release|package|install|billing|payment|production/u.test(lower)) return "release-package";
  return "command";
}

function scriptCapabilities(name, text) {
  const lower = `${name}\n${text}`.toLowerCase();
  const capabilities = ["local"];
  if (lower.includes("ci")) capabilities.push("ci");
  if (lower.includes("windows")) capabilities.push("windows");
  if (lower.includes("linux")) capabilities.push("linux");
  if (lower.includes("macos")) capabilities.push("macos");
  if (lower.includes("wsl")) capabilities.push("wsl");
  if (lower.includes("android")) capabilities.push(lower.includes("emulator") ? "android-emulator" : "android-device");
  if (lower.includes("ios")) capabilities.push(lower.includes("device") ? "ios-device" : "ios-simulator");
  if (lower.includes("browser") || lower.includes("playwright")) capabilities.push("browser");
  if (lower.includes("network") || lower.includes("lan")) capabilities.push("network");
  if (lower.includes("cloud")) capabilities.push("cloud");
  if (lower.includes("manual-required") || lower.includes("physical")) capabilities.push("manual-required");
  return uniqueSorted(capabilities);
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
    { type: "proof-finished", runId, proofId: proofContext.proofId, timestamp: startedAt, status },
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

function writeProofRunEnvelope({ root, runId, proofContext, harnessReport, pinned }) {
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

function baseProofRun({ root, runId, proofContext, status, exitCode, startedAt, endedAt, command, diagnosticCount, pinned = false }) {
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
  fs.writeFileSync(path.join(runDir, "attestation.json"), `${JSON.stringify(attestationFor(proofRun), null, 2)}\n`, "utf8");
  proofRun.artifacts = [
    "summary.md",
    "summary.json",
    "events.ndjson",
    "diagnostics.ndjson",
    "raw/stdout.log",
    "raw/stderr.log",
    "attestation.json",
  ]
    .map((name) => artifactRecord(root, runDir, name))
    .filter(Boolean);
  fs.writeFileSync(path.join(runDir, "proof-run.json"), `${JSON.stringify(proofRun, null, 2)}\n`, "utf8");
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
  const current = fs.existsSync(manifestPath) ? JSON.parse(fs.readFileSync(manifestPath, "utf8")) : { schemaVersion: 1, runs: [] };
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
  ].sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)));
  fs.writeFileSync(manifestPath, `${JSON.stringify(current, null, 2)}\n`, "utf8");
}

function listProofRuns(root) {
  const manifestPath = path.join(proofStorageRoot(root), PROOF_MANIFEST);
  if (!fs.existsSync(manifestPath)) return [];
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  return (manifest.runs ?? []).map((entry) => readProofRun(root, entry.runId)).filter(Boolean);
}

function latestProofRun(root, proofId) {
  return listProofRuns(root)
    .filter((run) => !proofId || run.proofId === proofId)
    .sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)))[0] ?? null;
}

function readProofRun(root, runId) {
  const proofRunPath = path.join(proofRunDir(root, runId), "proof-run.json");
  return fs.existsSync(proofRunPath) ? JSON.parse(fs.readFileSync(proofRunPath, "utf8")) : null;
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
  const runs = listProofRuns(root).sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)));
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
      if (Number.isFinite(ageMs) && ageMs > DEFAULT_PROOF_RETENTION.pruneAfterDays * 24 * 60 * 60 * 1000) remove.add(run.runId);
    }
  }
  for (const proofRuns of byProof.values()) {
    for (const run of proofRuns.slice(0, DEFAULT_PROOF_RETENTION.maxRunsPerProof)) keep.add(run.runId);
    for (const run of proofRuns.slice(DEFAULT_PROOF_RETENTION.maxRunsPerProof)) remove.add(run.runId);
  }
  for (const run of runs.filter((entry) => entry.status !== "passed").slice(0, DEFAULT_PROOF_RETENTION.maxFailedRuns)) keep.add(run.runId);
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
    .sort((left, right) => String(right.startedAt).localeCompare(String(left.startedAt)));
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

function createProofRunId(prefix) {
  const safePrefix = sanitizeSegment(prefix);
  const stamp = new Date().toISOString().replace(/[-:.TZ]/gu, "").slice(0, 14);
  return `${safePrefix}-${stamp}-${crypto.randomUUID().slice(0, 8)}`;
}

function sanitizeSegment(value) {
  return String(value ?? "proof").replace(/[^A-Za-z0-9._-]+/gu, "-").replace(/^-+|-+$/gu, "") || "proof";
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
  fs.writeFileSync(filePath, rows.map((row) => JSON.stringify(row)).join("\n") + (rows.length > 0 ? "\n" : ""), "utf8");
}

function adHocProofDefinition(proofId, collector) {
  return {
    id: proofId,
    title: proofId,
    family: collector === "manual-artifact" ? "manual-artifact" : "command",
    severity: "error",
    collector,
    capabilities: collector === "manual-artifact" ? ["manual-required"] : ["local"],
    docs: ["proof/INDEX.md#ad-hoc-proof"],
    appliesTo: ["workspace"],
    triggers: [],
  };
}

function stringListMatches(values, needle) {
  return values.some((value) => value === needle || value.endsWith(`:${needle}`) || value.includes(needle));
}

function splitList(value) {
  return String(value ?? "")
    .split(",")
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function stripNullish(value) {
  return Object.fromEntries(Object.entries(value).filter(([, entry]) => entry !== null && entry !== undefined));
}

function countBy(rows, key) {
  const result = {};
  for (const row of rows) result[row[key] ?? "unknown"] = (result[row[key] ?? "unknown"] ?? 0) + 1;
  return result;
}

function countCapabilities(scripts) {
  const result = {};
  for (const script of scripts) {
    for (const capability of script.capabilities) result[capability] = (result[capability] ?? 0) + 1;
  }
  return result;
}

function countReferences(scripts) {
  const result = {};
  for (const script of scripts) {
    for (const reference of script.references) result[reference] = (result[reference] ?? 0) + 1;
  }
  return result;
}

function claimViolation(proofId, code, message) {
  return { proofId, code, message, severity: "error" };
}
