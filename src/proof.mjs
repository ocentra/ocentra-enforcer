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

// PUBLIC-API-BUDGET-JUSTIFICATION: proof harness exports CLI, routing, storage, and claim helpers as one integration surface.
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

const SECRET_REDACTION_PATTERNS = [
  /\bgh[pousr]_[A-Za-z0-9_]{20,}\b/gu,
  /\bAKIA[0-9A-Z]{16}\b/gu,
  /\b(?:sk|pk)_(?:live|test)_[A-Za-z0-9]{20,}\b/gu,
  /\beyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\b/gu,
  /\b(?:secret|token|password|key)\b\s*[:=]\s*["'][A-Za-z0-9+/=_-]{16,}["']/giu,
  /-----BEGIN (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |OPENSSH |EC |DSA |)?PRIVATE KEY-----/gu,
];

export function loadProofRegistry(packRoot = DEFAULT_PACK_ROOT, profileName = null) {
  const registryPath = path.join(packRoot, "proof", "proofs.json");
  const baseRegistry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  const profileRegistry = profileName ? readProfileProofRegistry(packRoot, profileName) : null;
  const merged = profileRegistry
    ? {
        schemaVersion: Math.max(baseRegistry.schemaVersion ?? 1, profileRegistry.schemaVersion ?? 1),
        productName: baseRegistry.productName,
        proofs: mergeProofDefinitions(baseRegistry.proofs ?? [], profileRegistry.proofs ?? []),
      }
    : baseRegistry;
  return decodeProofRegistry(merged);
}

export function routeProofs(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRouteRequest(input);
  const root = path.resolve(args.root ?? process.cwd());
  const profileName = args.profile ?? "strict";
  const registry = loadProofRegistry(packRoot, profileName);
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
  const allMigrationRows = buildMigrationMatrix(scripts, Number.POSITIVE_INFINITY);
  return {
    ok: true,
    root,
    scriptsRoot: normalizeRel(root, scriptsRoot),
    totals,
    byFamily: countBy(scripts, "family"),
    byProofType: countArrayValues(scripts, "proofTypes"),
    byPlanBucket: countBy(scripts, "planBucket"),
    byMigrationTemplate: countByNested(scripts, (script) => script.migration.template),
    byCapability: countCapabilities(scripts),
    references: countReferences(scripts),
    claimSignals: countClaimSignals(scripts),
    migrationMatrix: allMigrationRows.slice(0, scriptLimit),
    migrationMatrixLimit: scriptLimit,
    omittedMigrationRowCount: Math.max(0, allMigrationRows.length - scriptLimit),
    scriptRowsIncluded: includeScripts,
    scriptLimit: includeScripts ? scriptLimit : 0,
    omittedScriptCount: Math.max(0, scripts.length - selectedScripts.length),
    scripts: selectedScripts,
  };
}

export function runProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRunArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const profile = args.profile ?? "strict";
  const registry = loadProofRegistry(packRoot, profile);
  const proofId = args.proofId ?? "ad-hoc-command-proof";
  const definition =
    registry.proofs.find((proof) => proof.id === proofId) ??
    adHocProofDefinition(proofId, args.command?.length ? "command" : "manual-artifact");
  const command = expandProofCommand(args.command?.length ? args.command : definition.commands?.[0] ?? [], {
    root,
    packRoot,
    profile,
    proofId,
    runId: args.runId ?? null,
  });
  const capability = args.capability ?? definition.capabilities?.[0] ?? "local";
  const runId = args.runId ?? createProofRunId(proofId);
  const git = gitState(root);
  const proofContext = {
    schemaVersion: 1,
    proofId,
    title: definition.title,
    family: definition.family,
    collector: definition.collector,
    profile,
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
    profile,
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
  const artifactAliases = {
    stdout: "raw/stdout.log",
    stderr: "raw/stderr.log",
    rawStdout: "raw/stdout.log",
    rawStderr: "raw/stderr.log",
  };
  const artifactName = artifactAliases[artifact] ?? artifact;
  const artifactRecord =
    run.artifacts.find((entry) => entry.name === artifactName) ??
    run.artifacts.find((entry) => entry.name === artifact) ??
    run.artifacts.find((entry) => entry.name === `${artifact}.md`) ??
    run.artifacts.find((entry) => entry.kind === artifactName || entry.kind === artifact || entry.name === artifact);
  if (!artifactRecord) return { ok: false, text: "", message: `Unknown proof artifact: ${artifact}` };
  const absolute = path.join(root, artifactRecord.path);
  const text = fs.existsSync(absolute) ? fs.readFileSync(absolute, "utf8") : "";
  return {
    ok: true,
    runId: run.runId,
    proofId: run.proofId,
    artifact,
    path: artifactRecord.path,
    text: redactSecrets(text).slice(0, args.limitBytes ?? 8000),
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
  return redactedJson({
    ok: true,
    root,
    bundle: {
      schemaVersion: 1,
      generatedAt: new Date().toISOString(),
      runs,
      note: "This is a manifest-only export. CI should upload artifacts separately instead of committing proof outputs.",
    },
  });
}

export function migrateLegacyProofs(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const profile = args.profile ?? projectProfileName(root);
  const scriptsRoot = path.join(root, args.scriptRoot ?? "scripts/test");
  const scripts = fs.existsSync(scriptsRoot)
    ? collectScriptFiles(scriptsRoot).map((filePath) => classifyProofScript(root, filePath))
    : [];
  const selectedScripts = scripts.filter((script) =>
    args.includeAllScripts ? true : script.name.includes("proof") || script.claimSemantics.claimKinds.length > 0,
  );
  const generatedRegistry = {
    schemaVersion: 1,
    productName: `${profile} proof profile`,
    proofs: selectedScripts.map((script) => legacyScriptProofDefinition({ profile, script })),
  };
  const decoded = decodeProofRegistry(generatedRegistry);
  const profileRoot = path.join(packRoot, "profiles", profile);
  const targetRegistryPath = path.join(profileRoot, "proofs.json");
  const targetScriptRoot = path.join(profileRoot, "legacy-scripts");
  const dryRun = !args.write;
  const result = {
    ok: true,
    dryRun,
    root,
    profile,
    scriptsRoot: normalizeRel(root, scriptsRoot),
    generatedProofCount: decoded.proofs.length,
    profileRegistryPath: normalizeRel(packRoot, targetRegistryPath),
    legacyScriptRoot: normalizeRel(packRoot, targetScriptRoot),
    deletionReadyWhen:
      "Package/workflow aliases point to proof run ids, generated profile proofs execute successfully, and proof parity/claim passes for the affected proof batch.",
    proofIds: decoded.proofs.slice(0, args.limit ?? 50).map((proof) => proof.id),
    omittedProofIdCount: Math.max(0, decoded.proofs.length - (args.limit ?? 50)),
  };
  if (dryRun) return result;

  fs.mkdirSync(path.dirname(targetRegistryPath), { recursive: true });
  fs.mkdirSync(targetScriptRoot, { recursive: true });
  fs.writeFileSync(targetRegistryPath, `${JSON.stringify(decoded, null, 2)}\n`, "utf8");
  for (const script of selectedScripts) {
    const source = repoAbsolute(root, script.path);
    const target = path.join(targetScriptRoot, script.path);
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(source, target);
  }
  return {
    ...result,
    copiedScriptCount: selectedScripts.length,
  };
}

export function importLegacyProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadProofRegistry(packRoot, args.profile ?? "strict");
  const proofId = args.proofId ?? "PROOF-LEGACY-ARTIFACT-IMPORT";
  const definition = registry.proofs.find((proof) => proof.id === proofId) ?? adHocProofDefinition(proofId, "file-hash");
  const legacy = collectLegacyProofArtifacts(root, args);
  const runId = args.runId ?? createProofRunId(proofId);
  const startedAt = new Date().toISOString();
  const git = gitState(root);
  const status = legacy.artifacts.length === 0 ? "failed" : legacy.failedArtifacts.length > 0 ? "failed" : "passed";
  const diagnostics = legacyDiagnostics({ runId, proofId, legacy });
  const proofContext = {
    schemaVersion: 1,
    proofId,
    title: definition.title,
    family: definition.family,
    collector: definition.collector,
    profile: args.profile ?? "strict",
    root,
    scope: {
      files: legacy.sourcePaths,
      plan: null,
      capability: "local",
    },
    git,
    claimsProved: uniqueSorted([...(definition.claimsProved ?? []), ...legacy.claimsProved]),
    claimsNotProved: uniqueSorted([...(definition.claimsNotProved ?? []), ...legacy.claimsNotProved]),
  };

  const preview = {
    ok: status === "passed",
    dryRun: true,
    root,
    proofRun: {
      ...proofContext,
      runId,
      status,
      ok: status === "passed",
      exitCode: status === "passed" ? 0 : 1,
      startedAt,
      endedAt: startedAt,
      command: ["legacy-import", ...legacy.sourcePaths],
      diagnosticCount: diagnostics.length,
      pinned: false,
      retention: DEFAULT_PROOF_RETENTION,
      artifacts: [],
      legacy,
    },
    diagnostics,
  };
  if (args.dryRun) return preview;

  const runDir = proofRunDir(root, runId);
  fs.mkdirSync(path.join(runDir, "artifacts", "legacy"), { recursive: true });
  for (const artifact of legacy.artifacts) {
    const target = path.join(runDir, "artifacts", "legacy", sanitizeRelativeArtifactName(artifact.path));
    fs.mkdirSync(path.dirname(target), { recursive: true });
    fs.copyFileSync(repoAbsolute(root, artifact.path), target);
  }
  fs.writeFileSync(
    path.join(runDir, "artifacts", "legacy-manifest.json"),
    `${JSON.stringify({ schemaVersion: 1, proofId, runId, legacy }, null, 2)}\n`,
    "utf8",
  );
  writeNdjson(path.join(runDir, "diagnostics.ndjson"), diagnostics);
  writeNdjson(path.join(runDir, "events.ndjson"), [
    { type: "proof-started", runId, proofId, timestamp: startedAt, source: "legacy-import" },
    { type: "legacy-artifacts-imported", runId, proofId, timestamp: startedAt, artifactCount: legacy.artifacts.length },
    { type: "proof-finished", runId, proofId, timestamp: startedAt, status },
  ]);
  const proofRun = baseProofRun({
    root,
    runId,
    proofContext,
    status,
    exitCode: status === "passed" ? 0 : 1,
    startedAt,
    endedAt: startedAt,
    command: ["legacy-import", ...legacy.sourcePaths],
    diagnosticCount: diagnostics.length,
  });
  proofRun.legacy = legacy;
  writeProofFiles(root, runDir, proofRun);
  updateProofManifest(root, proofRun);
  pruneProofRuns({ root });
  return { ok: proofRun.status === "passed", dryRun: false, root, proofRun, diagnostics };
}

export function proofParity(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const proofId = args.proofId ?? "PROOF-LEGACY-ARTIFACT-IMPORT";
  const legacy = collectLegacyProofArtifacts(root, args);
  const run = args.runId ? readProofRun(root, args.runId) : latestProofRun(root, proofId);
  const runLegacy = run?.legacy ?? null;
  const legacyHashes = new Map(legacy.artifacts.map((artifact) => [artifact.path, artifact.sha256]));
  const importedHashes = new Map((runLegacy?.artifacts ?? []).map((artifact) => [artifact.path, artifact.sha256]));
  const missingInImported = [...legacyHashes]
    .filter(([artifactPath, hash]) => importedHashes.get(artifactPath) !== hash)
    .map(([artifactPath]) => artifactPath);
  const extraImported = [...importedHashes.keys()].filter((artifactPath) => !legacyHashes.has(artifactPath));
  const missingClaimsProved = legacy.claimsProved.filter((claim) => !(run?.claimsProved ?? []).includes(claim));
  const missingClaimsNotProved = legacy.claimsNotProved.filter((claim) => !(run?.claimsNotProved ?? []).includes(claim));
  const hasRun = Boolean(run);
  const comparable = legacy.artifacts.length > 0 && hasRun && runLegacy !== null;
  const equivalent =
    comparable &&
    missingInImported.length === 0 &&
    missingClaimsProved.length === 0 &&
    missingClaimsNotProved.length === 0;
  const coverage = equivalent ? "equivalent" : comparable ? "weaker" : "not-comparable";
  const deletionReady = equivalent && run.status === "passed" && legacy.failedArtifacts.length === 0;
  return {
    ok: deletionReady,
    root,
    proofId,
    runId: run?.runId ?? null,
    coverage,
    deletionReady,
    nextStep: deletionReady
      ? "This bounded legacy proof artifact set can be rewired to Enforcer and the matching old script batch can be deleted after wrapper/CI parity."
      : "Do not delete old proof scripts for this set. Import or rerun legacy proof artifacts until parity is equivalent or stricter.",
    legacy: {
      artifactCount: legacy.artifacts.length,
      failedArtifactCount: legacy.failedArtifacts.length,
      claimsProved: legacy.claimsProved,
      claimsNotProved: legacy.claimsNotProved,
    },
    imported: {
      artifactCount: runLegacy?.artifacts?.length ?? 0,
      status: run?.status ?? null,
      claimsProved: run?.claimsProved ?? [],
      claimsNotProved: run?.claimsNotProved ?? [],
    },
    differences: {
      missingInImported,
      extraImported,
      missingClaimsProved,
      missingClaimsNotProved,
      statusFailures: legacy.failedArtifacts,
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
    case "migrate-legacy":
      result = migrateLegacyProofs(parsed, packRoot);
      break;
    case "import-legacy":
      result = importLegacyProof(parsed, packRoot);
      break;
    case "parity":
      result = proofParity(parsed);
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
  if (command === "import-legacy") {
    return `Imported legacy proof ${result.proofRun.proofId} ${result.proofRun.status}: ${result.proofRun.runId}`;
  }
  if (command === "migrate-legacy") {
    return `Generated ${result.generatedProofCount} legacy proof definition(s) for profile ${result.profile}: dryRun=${result.dryRun}`;
  }
  if (command === "parity") {
    return `Proof parity ${result.coverage}: deletionReady=${result.deletionReady}`;
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
    legacyPaths: [],
    dryRun: false,
    status: null,
    tags: [],
    json: false,
    pin: false,
    prReady: false,
    allowDirty: false,
    claimId: null,
    write: false,
    scriptRoot: null,
    includeAllScripts: false,
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
    else if (token === "--legacy-path" || token === "--legacy-paths") parsed.legacyPaths = splitList(tokens[++index] ?? "");
    else if (token === "--status") parsed.status = tokens[++index] ?? null;
    else if (token === "--tag") parsed.tags.push(tokens[++index] ?? "");
    else if (token === "--claim-id") parsed.claimId = tokens[++index] ?? null;
    else if (token === "--script-root") parsed.scriptRoot = tokens[++index] ?? null;
    else if (token === "--json") parsed.json = true;
    else if (token === "--dry-run") parsed.dryRun = true;
    else if (token === "--write") parsed.write = true;
    else if (token === "--include-all-scripts") parsed.includeAllScripts = true;
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

function readProfileProofRegistry(packRoot, profileName) {
  const registryPath = path.join(packRoot, "profiles", profileName, "proofs.json");
  if (!fs.existsSync(registryPath)) return null;
  return JSON.parse(fs.readFileSync(registryPath, "utf8"));
}

function mergeProofDefinitions(baseProofs, profileProofs) {
  const merged = new Map();
  for (const proof of baseProofs) merged.set(proof.id, proof);
  for (const proof of profileProofs) merged.set(proof.id, proof);
  return [...merged.values()];
}

function expandProofCommand(command, context) {
  return command.map((part) =>
    String(part)
      .replaceAll("$node", process.execPath)
      .replaceAll("$root", context.root)
      .replaceAll("$packRoot", context.packRoot)
      .replaceAll("$profile", context.profile)
      .replaceAll("$proofId", context.proofId)
      .replaceAll("$runId", context.runId ?? ""),
  );
}

function projectProfileName(root) {
  return normalizeProofSlug(path.basename(root));
}

function legacyScriptProofDefinition({ profile, script }) {
  const proofId = `${profile}.${normalizeProofSlug(script.name.replace(/\.mjs$/u, ""))}`;
  return {
    id: proofId,
    title: `Legacy migrated proof: ${script.name}`,
    family: script.family,
    severity: "error",
    appliesTo: ["profile:legacy-project", `plan:${script.planBucket}`, "kind:proof-script"],
    triggers: uniqueSorted(["kind:proof-script", ...script.proofTypes, ...script.capabilities.map((capability) => `capability:${capability}`)]),
    languages: ["common"],
    capabilities: script.capabilities,
    collector: "command",
    docs: ["proof/INDEX.md#legacy-script-migration", "docs/PROOF_SYSTEM_DESIGN.md#deterministic-migration-sequence"],
    commands: [
      [
        "$node",
        "$packRoot/scripts/profile-proof-runner.mjs",
        "--root",
        "$root",
        "--profile",
        "$profile",
        "--script",
        script.path,
      ],
    ],
    requiredArtifacts: ["proof-run.json", "summary.md", "diagnostics.ndjson"],
    requiredPaths: [],
    claimsProved: [
      `Migrated legacy proof script ${script.path} executed through Enforcer profile ${profile}.`,
      ...legacyScriptClaimSummaries(script, "proved"),
    ],
    claimsNotProved: legacyScriptClaimSummaries(script, "not-proved"),
    ciSupport: script.capabilities.includes("ci"),
    deviceSupport:
      script.capabilities.includes("android-device") ||
      script.capabilities.includes("ios-device") ||
      script.capabilities.includes("manual-required"),
  };
}

function legacyScriptClaimSummaries(script, kind) {
  const summaries = [];
  if (kind === "proved") {
    if (script.claimSemantics.hasExpectationRows) summaries.push("Referenced expectation matrix rows are part of the proof scope.");
    if (script.signals.spawn) summaries.push("Legacy command execution remains captured under bounded Enforcer artifacts.");
    if (script.signals.writesProof) summaries.push("Legacy artifact outputs are captured as proof evidence.");
  } else {
    if (script.capabilities.includes("manual-required")) summaries.push("Manual or physical device behavior is not silently claimed without collected evidence.");
    if (script.claimSemantics.dependsOnPriorProof) summaries.push("Prior proof dependencies must be present and current before this proof can close a plan.");
  }
  return uniqueSorted(summaries);
}

function normalizeProofSlug(value) {
  return String(value ?? "proof")
    .toLowerCase()
    .replace(/[^a-z0-9]+/gu, "-")
    .replace(/^-+|-+$/gu, "") || "proof";
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
  const family = scriptFamily(name, text);
  const proofTypes = scriptProofTypes(name, text);
  const migration = scriptMigrationPlan({ name, text, family, proofTypes, capabilities });
  return {
    path: rel,
    name,
    family,
    planBucket: scriptPlanBucket(name, text),
    proofTypes,
    claimSemantics: scriptClaimSemantics(text),
    migration,
    capabilities,
    signals: {
      spawn: /\bspawn(?:Sync)?\b|\bexec(?:File|Sync)?\b/u.test(text),
      writesProof: /\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text),
      readsProof: readsPriorProofText(text),
      manualOrDevice: /manual-required|physical|ANDROID_SERIAL|\badb\b|ios|simulator|device/iu.test(text),
      importsBuiltOrSchemaParse: /dist\/|await import|Schema\.parse|\.parse\(/u.test(text),
      claimsProved: /claimsProved/u.test(text),
      claimsNotProved: /claimsNotProved|mustNotClaim|notProved/u.test(text),
      rowsCovered: /rowsCovered|matrixRows|expectations/u.test(text),
      writesMarkdown: /\.md['"]|summary\.md|snapshot\.md/u.test(text),
      screenshots: /screenshot|png|jpg|image/iu.test(text),
      adb: /\badb\b|ANDROID_SERIAL/u.test(text),
      playwright: /playwright|\.spec\./u.test(text),
      cargo: /cargo /u.test(text),
      npm: /npm /u.test(text),
      vitest: /vitest/u.test(text),
    },
    references,
    outputRoots: uniqueSorted([...text.matchAll(/(?:test-results|output\/|docs\/proof)[^'"`\s,)]+/gu)].map((match) => match[0])),
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

function scriptPlanBucket(name, text) {
  const lowerName = name.toLowerCase();
  const lowerText = text.toLowerCase();
  const knownBuckets = [
    ["app-install", "app-install-purchase"],
    ["app-game", "app-game"],
    ["browser-game", "browser-game"],
    ["browser", "browser"],
    ["screen-ai", "screen-ai"],
    ["screen-vlm", "screen"],
    ["screen-capture", "screen"],
    ["screen-live", "screen"],
    ["screen-", "screen"],
    ["v0-8", "v0-8-platform-adapters"],
    ["v0-9", "v0-9-household-lan"],
    ["child-android", "child-android"],
    ["child-ios", "child-ios"],
    ["child-macos", "child-macos"],
    ["eventing-network", "eventing-network"],
    ["eventing", "eventing"],
    ["parent-desktop", "parent-desktop"],
    ["production-release", "production-release"],
    ["production-support", "production-support"],
    ["network-remote", "network-remote-delivery"],
    ["network-live", "network-live-capture"],
    ["network-evidence", "network-evidence"],
    ["network-windows", "network-windows"],
    ["tracking", "tracking"],
    ["social", "social"],
    ["local-ai", "local-ai"],
    ["provider-secret", "provider-secret"],
    ["billing-entitlement", "billing-entitlement"],
    ["lan-ai", "lan-ai"],
    ["managed-browser", "managed-browser"],
  ];
  for (const [needle, bucket] of knownBuckets) {
    if (lowerName.includes(needle)) return bucket;
  }
  for (const [needle, bucket] of knownBuckets) {
    if (lowerText.includes(`docs/plans/${needle}`) || lowerText.includes(`docs/expectations/${needle}`)) return bucket;
  }
  return name.replace(/\.mjs$/u, "").split("-").slice(0, 2).join("-") || "unclassified";
}

function scriptProofTypes(name, text) {
  const lowerName = name.toLowerCase();
  const lowerText = text.toLowerCase();
  const types = [];
  if (/android|ios|xcode|device|simulator/u.test(lowerName) || /ANDROID_SERIAL|\badb\b/u.test(text)) types.push("device-execution");
  if (/manual-required|physical/u.test(lowerText) || /screenshot|snapshot\.md/iu.test(text)) types.push("manual-evidence");
  if (/claimsproved|claimsnotproved|mustnotclaim|notproved/u.test(lowerText)) types.push("claim-integrity");
  if (/rowscovered|matrixrows|docs\/expectations|pre-ai-proof-matrix/u.test(lowerText)) types.push("expectation-matrix");
  if (readsPriorProofText(text)) types.push("proof-composition");
  if (/parity|contract|schema|codec|boundary/u.test(lowerName) || /Schema\.parse|decode[A-Z]\w+\(/u.test(text)) types.push("contract-parity");
  if (/event|network|lan|message|websocket|bridge/u.test(lowerName)) types.push("runtime-event-contract");
  if (/release|package|install|billing|payment|production/u.test(lowerName)) types.push("release-readiness");
  if (/sarif|codeql|security|secret|audit/u.test(lowerName) || /sarif|codeql|pip-audit|cargo audit/u.test(lowerText)) types.push("security-report");
  if (/junit|pytest|vitest|jest|playwright|cargo-test|npm-test|\.spec\.|\.test\./u.test(lowerName) || /vitest|playwright|cargo test|npm run test/u.test(lowerText)) types.push("test-report");
  if (/\bspawn(?:Sync)?\b|\bexec(?:File|Sync)?\b/u.test(text)) types.push("command-execution");
  if (/\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text) || /output\/|docs\/proof|summary\.md/u.test(lowerText)) types.push("artifact-snapshot");
  return uniqueSorted(types.length > 0 ? types : ["command-execution"]);
}

function scriptClaimSemantics(text) {
  const claimKinds = [];
  if (/claimsProved/u.test(text)) claimKinds.push("explicit-claims-proved");
  if (/claimsNotProved|mustNotClaim|notProved/u.test(text)) claimKinds.push("explicit-non-claims");
  if (/rowsCovered|matrixRows|docs\/expectations/u.test(text)) claimKinds.push("expectation-row-coverage");
  if (readsPriorProofText(text)) claimKinds.push("prior-proof-dependency");
  if (/\b(?:writeFile|writeFileSync|appendFile|appendFileSync|writeJson)\s*\(/u.test(text)) claimKinds.push("artifact-producing");
  if (/manual-required|physical|ANDROID_SERIAL|\badb\b|ios|simulator|device/iu.test(text)) claimKinds.push("capability-gated");
  return {
    claimKinds: uniqueSorted(claimKinds),
    hasExplicitClaimList: /claimsProved|claimsNotProved/u.test(text),
    hasNonClaimBoundary: /claimsNotProved|mustNotClaim|notProved/u.test(text),
    hasExpectationRows: /rowsCovered|matrixRows|docs\/expectations/u.test(text),
    dependsOnPriorProof: readsPriorProofText(text),
  };
}

function readsPriorProofText(text) {
  return /readJson|readFile(?:Sync)?\s*\([^)]*(?:proof|test-results)|loadProof|proofManifest/u.test(text);
}

function scriptMigrationPlan({ name, text, family, proofTypes, capabilities }) {
  const typeSet = new Set(proofTypes);
  const capabilitySet = new Set(capabilities);
  const lower = `${name}\n${text}`.toLowerCase();
  if (typeSet.has("device-execution") || capabilitySet.has("manual-required")) {
    return {
      template: "DeviceProof<Capability, DeviceSelector, ArtifactPlan>",
      mode: "capability-gated",
      deletionGate: "new proof records device identity, unavailable/waived state, required artifacts, and explicit non-claims",
    };
  }
  if (typeSet.has("proof-composition") && typeSet.has("claim-integrity")) {
    return {
      template: "ProofBundleClaim<RequiredProofIds, ClosureMatrix>",
      mode: "compose-existing-proofs",
      deletionGate: "new proof rejects stale/missing prior runs and deleted artifact paths",
    };
  }
  if (typeSet.has("contract-parity")) {
    return {
      template: "ContractParityProof<Authority, Mirror, DriftTests>",
      mode: "replace-with-contract-definition",
      deletionGate: "new proof checks authority source, generated mirrors, drift tests, and copied-value non-claims",
    };
  }
  if (typeSet.has("runtime-event-contract")) {
    return {
      template: "RuntimeEventProof<Producer, Transport, Consumer>",
      mode: "replace-with-runtime-contract-definition",
      deletionGate: "new proof captures command output plus event/transport assertions and authentic-runtime claims",
    };
  }
  if (typeSet.has("release-readiness") || family === "release-package" || /production/u.test(lower)) {
    return {
      template: "ReleaseReadinessProof<Workflow, Artifact, ManualGate>",
      mode: "replace-with-release-definition",
      deletionGate: "new proof separates CI-reproducible artifacts from manual/platform gates",
    };
  }
  if (typeSet.has("security-report")) {
    return {
      template: "SecurityReportProof<SarifOrAudit, Policy>",
      mode: "replace-with-parser-definition",
      deletionGate: "new proof parses SARIF/audit output and ties findings to commit",
    };
  }
  if (typeSet.has("test-report")) {
    return {
      template: "StructuredTestReportProof<Runner, ReportParser>",
      mode: "replace-with-parser-definition",
      deletionGate: "new proof parses nonzero tests, failures, skipped/focused tests, and report artifacts",
    };
  }
  return {
    template: "CommandProof<CommandPlan, AssertionPlan>",
    mode: "replace-with-command-definition",
    deletionGate: "new proof captures command, bounded logs, structured diagnostics, artifacts, and explicit claims",
  };
}

function buildMigrationMatrix(scripts, limit = 20) {
  const rows = new Map();
  for (const script of scripts) {
    const key = `${script.planBucket}\u0000${script.migration.template}`;
    const current = rows.get(key) ?? {
      planBucket: script.planBucket,
      scriptCount: 0,
      families: {},
      proofTypes: {},
      capabilities: {},
      migrationTemplate: script.migration.template,
      migrationMode: script.migration.mode,
      deletionGate: script.migration.deletionGate,
      representativeScripts: [],
    };
    current.scriptCount += 1;
    current.families[script.family] = (current.families[script.family] ?? 0) + 1;
    for (const proofType of script.proofTypes) current.proofTypes[proofType] = (current.proofTypes[proofType] ?? 0) + 1;
    for (const capability of script.capabilities) current.capabilities[capability] = (current.capabilities[capability] ?? 0) + 1;
    if (current.representativeScripts.length < 5) current.representativeScripts.push(script.path);
    rows.set(key, current);
  }
  return [...rows.values()]
    .sort((left, right) => right.scriptCount - left.scriptCount || left.planBucket.localeCompare(right.planBucket))
    .slice(0, limit);
}

const DEFAULT_LEGACY_PROOF_ROOTS = ["test-results", "output", "docs/proof"];
const LEGACY_ARTIFACT_EXTENSIONS = new Set([
  ".json",
  ".md",
  ".txt",
  ".log",
  ".xml",
  ".sarif",
  ".ndjson",
  ".png",
  ".jpg",
  ".jpeg",
  ".webm",
  ".zip",
]);

function collectLegacyProofArtifacts(root, args = {}) {
  const explicitPaths = args.legacyPaths?.length ? args.legacyPaths : [];
  const sourceFiles = uniqueSorted(
    (explicitPaths.length > 0
      ? explicitPaths
      : DEFAULT_LEGACY_PROOF_ROOTS.filter((entry) => fs.existsSync(repoAbsolute(root, entry)))
    ).flatMap((entry) => collectLegacyArtifactFiles(root, entry)),
  ).slice(0, Number.isFinite(args.limit) ? Math.max(0, args.limit) : 500);
  const artifacts = sourceFiles.map((file) => analyzeLegacyArtifact(root, file));
  const claimsProved = uniqueSorted(artifacts.flatMap((artifact) => artifact.claimsProved));
  const claimsNotProved = uniqueSorted(artifacts.flatMap((artifact) => artifact.claimsNotProved));
  const failedArtifacts = artifacts
    .filter((artifact) => ["failed", "manual-required", "unavailable"].includes(artifact.status))
    .map((artifact) => ({
      path: artifact.path,
      status: artifact.status,
      reason: artifact.statusReason,
    }));
  return {
    schemaVersion: 1,
    collectedAt: new Date().toISOString(),
    sourceRoots: explicitPaths.length > 0 ? explicitPaths : DEFAULT_LEGACY_PROOF_ROOTS,
    sourcePaths: artifacts.map((artifact) => artifact.path),
    artifactCount: artifacts.length,
    artifacts,
    failedArtifacts,
    claimsProved,
    claimsNotProved,
    fingerprint: legacyFingerprint(artifacts),
  };
}

function collectLegacyArtifactFiles(root, entry) {
  const absolute = repoAbsolute(root, entry);
  if (!fs.existsSync(absolute)) return [];
  const stat = fs.statSync(absolute);
  if (stat.isFile()) return isLegacyProofArtifactFile(absolute) ? [normalizeRel(root, absolute)] : [];
  if (!stat.isDirectory()) return [];
  const files = [];
  const stack = [absolute];
  while (stack.length > 0) {
    const current = stack.pop();
    for (const child of fs.readdirSync(current, { withFileTypes: true })) {
      const childPath = path.join(current, child.name);
      if (child.isDirectory()) {
        if ([".git", ".enforce", "node_modules", "target", "dist", "build"].includes(child.name)) continue;
        stack.push(childPath);
      } else if (child.isFile() && isLegacyProofArtifactFile(childPath)) {
        files.push(normalizeRel(root, childPath));
      }
    }
  }
  return files;
}

function isLegacyProofArtifactFile(filePath) {
  return LEGACY_ARTIFACT_EXTENSIONS.has(path.extname(filePath).toLowerCase());
}

function analyzeLegacyArtifact(root, relPath) {
  const absolute = repoAbsolute(root, relPath);
  const content = fs.readFileSync(absolute);
  const ext = path.extname(absolute).toLowerCase();
  const text = [".json", ".md", ".txt", ".log", ".xml", ".sarif", ".ndjson"].includes(ext) ? content.toString("utf8") : "";
  const json = ext === ".json" || ext === ".sarif" ? parseJsonMaybe(text) : null;
  const statusInfo = inferLegacyStatus(json, text);
  return {
    path: normalizeRel(root, absolute),
    kind: ext.replace(/^\./u, "") || "artifact",
    sha256: crypto.createHash("sha256").update(content).digest("hex"),
    byteLength: content.byteLength,
    status: statusInfo.status,
    statusReason: statusInfo.reason,
    claimsProved: uniqueSorted([...claimsFromJson(json, "proved"), ...claimsFromMarkdown(text, "proved")]),
    claimsNotProved: uniqueSorted([...claimsFromJson(json, "not-proved"), ...claimsFromMarkdown(text, "not-proved")]),
  };
}

function inferLegacyStatus(json, text) {
  const value = json && typeof json === "object" ? json : {};
  const status = String(value.status ?? value.result ?? "").toLowerCase();
  if (value.ok === false || value.passed === false || value.success === false || ["failed", "fail", "error"].includes(status)) {
    return { status: "failed", reason: "legacy artifact reports a failed status" };
  }
  if (["manual-required", "manual_required"].includes(status) || /manual-required/iu.test(text)) {
    return { status: "manual-required", reason: "legacy artifact reports manual-required evidence" };
  }
  if (["unavailable", "skipped"].includes(status)) {
    return { status: "unavailable", reason: `legacy artifact reports ${status}` };
  }
  if (value.ok === true || value.passed === true || value.success === true || ["passed", "pass", "ok"].includes(status)) {
    return { status: "passed", reason: "legacy artifact reports a passing status" };
  }
  return { status: "present", reason: "legacy artifact exists but does not expose a standard status field" };
}

function parseJsonMaybe(text) {
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function claimsFromJson(json, kind) {
  if (!json || typeof json !== "object") return [];
  const keys =
    kind === "proved"
      ? ["claimsProved", "provedClaims", "claims", "rowsCovered"]
      : ["claimsNotProved", "claimsNotProven", "notProved", "mustNotClaim", "nonClaims"];
  return keys.flatMap((key) => normalizeClaimList(json[key]));
}

function normalizeClaimList(value) {
  if (Array.isArray(value)) return value.flatMap((entry) => normalizeClaimList(entry));
  if (value && typeof value === "object") {
    if (typeof value.claim === "string") return [value.claim];
    if (typeof value.id === "string") return [value.id];
    if (typeof value.name === "string") return [value.name];
    return [];
  }
  return typeof value === "string" && value.trim().length > 0 ? [value.trim()] : [];
}

function claimsFromMarkdown(text, kind) {
  if (!text) return [];
  const headerPattern =
    kind === "proved"
      ? /claims?\s+proved|proved\s+claims|what\s+this\s+proves/iu
      : /claims?\s+not\s+proved|non-claims?|what\s+this\s+does\s+not\s+prove|not\s+proved/iu;
  const lines = text.split(/\r?\n/u);
  const claims = [];
  let capturing = false;
  for (const line of lines) {
    if (/^\s{0,3}#{1,6}\s+/u.test(line)) {
      capturing = headerPattern.test(line);
      continue;
    }
    if (!capturing) continue;
    const bullet = /^\s*(?:[-*]|\d+\.)\s+(.+?)\s*$/u.exec(line);
    if (bullet) claims.push(bullet[1]);
    else if (line.trim().length === 0) continue;
    else if (/^\s{0,3}#{1,6}\s+/u.test(line)) capturing = false;
  }
  return claims;
}

function legacyFingerprint(artifacts) {
  const hash = crypto.createHash("sha256");
  for (const artifact of artifacts) {
    hash.update(`${artifact.path}\0${artifact.sha256}\0${artifact.byteLength}\n`);
  }
  return `sha256:${hash.digest("hex")}`;
}

function legacyDiagnostics({ runId, proofId, legacy }) {
  if (legacy.artifacts.length === 0) {
    return [
      {
        runId,
        proofId,
        severity: "error",
        ruleId: "PROOF-LEGACY-MISSING",
        message: "No legacy proof artifacts were found for the selected roots.",
        file: ".",
        line: 1,
      },
    ];
  }
  return legacy.failedArtifacts.map((artifact) => ({
    runId,
    proofId,
    severity: "error",
    ruleId: "PROOF-LEGACY-STATUS",
    message: `${artifact.path} is not a passing legacy proof artifact: ${artifact.reason}`,
    file: artifact.path,
    line: 1,
  }));
}

function sanitizeRelativeArtifactName(relPath) {
  return toPosix(relPath).replace(/^[A-Za-z]:/u, "").replace(/^\/+/u, "").replace(/[^A-Za-z0-9._/-]+/gu, "-");
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
  proofRun.artifacts = collectProofArtifactRecords(root, runDir);
  fs.writeFileSync(path.join(runDir, "proof-run.json"), `${JSON.stringify(proofRun, null, 2)}\n`, "utf8");
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

function countByNested(rows, accessor) {
  const result = {};
  for (const row of rows) {
    const key = accessor(row) ?? "unknown";
    result[key] = (result[key] ?? 0) + 1;
  }
  return result;
}

function countArrayValues(rows, key) {
  const result = {};
  for (const row of rows) {
    for (const value of row[key] ?? []) result[value] = (result[value] ?? 0) + 1;
  }
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

function countClaimSignals(scripts) {
  return {
    explicitClaimsProved: scripts.filter((script) => script.claimSemantics.hasExplicitClaimList).length,
    explicitNonClaimBoundary: scripts.filter((script) => script.claimSemantics.hasNonClaimBoundary).length,
    expectationRows: scripts.filter((script) => script.claimSemantics.hasExpectationRows).length,
    priorProofDependencies: scripts.filter((script) => script.claimSemantics.dependsOnPriorProof).length,
    artifactProducing: scripts.filter((script) => script.claimSemantics.claimKinds.includes("artifact-producing")).length,
    capabilityGated: scripts.filter((script) => script.claimSemantics.claimKinds.includes("capability-gated")).length,
  };
}

function claimViolation(proofId, code, message) {
  return { proofId, code, message, severity: "error" };
}
