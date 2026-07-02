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
import { formatProofReport, parseProofCli, stripNullish } from "./proof-cli.mjs";
import {
  buildMigrationMatrix,
  classifyProofScript,
  collectLegacyProofArtifacts,
  collectScriptFiles,
  compactProofDefinition,
  countArrayValues,
  countBy,
  countByNested,
  countCapabilities,
  countClaimSignals,
  countReferences,
  describeProofRouteScope,
  expandProofCommand,
  legacyDiagnostics,
  legacyScriptProofDefinition,
  mergeProofDefinitions,
  projectProfileName,
  proofFamilyKeys,
  proofMatchesRoute,
  readProfileProofRegistry,
  sanitizeRelativeArtifactName,
} from "./proof-legacy.mjs";
import {
  baseProofRun,
  createProofRunId,
  DEFAULT_PROOF_RETENTION,
  gitState,
  latestProofRun,
  listProofRuns,
  proofHarnessConfig,
  proofRunDir,
  proofStorageRoot,
  pruneProofRuns,
  readProofDiagnostics,
  readProofRun,
  redactSecrets,
  redactedJson,
  PROOF_STORAGE_DIR,
  updateProofManifest,
  writeManualProofRun,
  writeNdjson,
  writeProofFiles,
  writeProofRunEnvelope,
} from "./proof-storage.mjs";
import { runHarness } from "./harness.mjs";
import { normalizeRel, repoAbsolute, uniqueSorted } from "./path-utils.mjs";

const DEFAULT_PACK_ROOT = path.resolve(
  path.join(path.dirname(fileURLToPath(import.meta.url)), ".."),
);

function loadProofRegistry(
  packRoot = DEFAULT_PACK_ROOT,
  profileName = null,
) {
  const registryPath = path.join(packRoot, "proof", "proofs.json");
  const baseRegistry = JSON.parse(fs.readFileSync(registryPath, "utf8"));
  const profileRegistry = profileName
    ? readProfileProofRegistry(packRoot, profileName)
    : null;
  const merged = profileRegistry
    ? {
        schemaVersion: Math.max(
          baseRegistry.schemaVersion ?? 1,
          profileRegistry.schemaVersion ?? 1,
        ),
        productName: baseRegistry.productName,
        proofs: mergeProofDefinitions(
          baseRegistry.proofs ?? [],
          profileRegistry.proofs ?? [],
        ),
      }
    : baseRegistry;
  return decodeProofRegistry(merged);
}

function routeProofs(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRouteRequest(input);
  const profileName = args.profile ?? "strict";
  const registry = loadProofRegistry(packRoot, profileName);
  const explicitProofId = args.proofId ?? null;
  const familyKeys = explicitProofId ? new Set() : proofFamilyKeys(args);
  const definitions = explicitProofId
    ? registry.proofs.filter((proof) => proof.id === explicitProofId)
    : registry.proofs.filter((proof) =>
        proofMatchesRoute(proof, args, familyKeys),
      );

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

function inventoryProofs(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const scriptsRoot = path.join(root, "scripts", "test");
  const scripts = fs.existsSync(scriptsRoot)
    ? collectScriptFiles(scriptsRoot).map((filePath) =>
        classifyProofScript(root, filePath),
      )
    : [];
  const totals = {
    scripts: scripts.length,
    proofNamed: scripts.filter((script) => script.name.includes("proof")).length,
    spawnCommands: scripts.filter((script) => script.signals.spawn).length,
    writesProof: scripts.filter((script) => script.signals.writesProof).length,
    readsProof: scripts.filter((script) => script.signals.readsProof).length,
    manualOrDevice: scripts.filter((script) => script.signals.manualOrDevice).length,
    importsBuiltOrSchemaParse: scripts.filter(
      (script) => script.signals.importsBuiltOrSchemaParse,
    ).length,
  };
  const includeScripts = Boolean(args.includeScripts);
  const scriptLimit = Number.isFinite(args.limit)
    ? Math.max(0, args.limit)
    : 20;
  const selectedScripts = includeScripts ? scripts.slice(0, scriptLimit) : [];
  const allMigrationRows = buildMigrationMatrix(
    scripts,
    Number.POSITIVE_INFINITY,
  );
  return {
    ok: true,
    root,
    scriptsRoot: normalizeRel(root, scriptsRoot),
    totals,
    byFamily: countBy(scripts, "family"),
    byProofType: countArrayValues(scripts, "proofTypes"),
    byPlanBucket: countBy(scripts, "planBucket"),
    byMigrationTemplate: countByNested(
      scripts,
      (script) => script.migration.template,
    ),
    byCapability: countCapabilities(scripts),
    references: countReferences(scripts),
    claimSignals: countClaimSignals(scripts),
    migrationMatrix: allMigrationRows.slice(0, scriptLimit),
    migrationMatrixLimit: scriptLimit,
    omittedMigrationRowCount: Math.max(
      0,
      allMigrationRows.length - scriptLimit,
    ),
    scriptRowsIncluded: includeScripts,
    scriptLimit: includeScripts ? scriptLimit : 0,
    omittedScriptCount: Math.max(0, scripts.length - selectedScripts.length),
    scripts: selectedScripts,
  };
}

function adHocProofDefinition(proofId, collector) {
  return {
    id: proofId,
    title: proofId,
    family: collector === "manual-artifact" ? "manual-artifact" : "command",
    severity: "error",
    collector,
    capabilities:
      collector === "manual-artifact" ? ["manual-required"] : ["local"],
    docs: ["proof/INDEX.md#ad-hoc-proof"],
    appliesTo: ["workspace"],
    triggers: [],
  };
}

function runProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofRunArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const profile = args.profile ?? "strict";
  const registry = loadProofRegistry(packRoot, profile);
  const proofId = args.proofId ?? "ad-hoc-command-proof";
  const definition =
    registry.proofs.find((proof) => proof.id === proofId) ??
    adHocProofDefinition(
      proofId,
      args.command?.length ? "command" : "manual-artifact",
    );
  const command = expandProofCommand(
    args.command?.length ? args.command : definition.commands?.[0] ?? [],
    {
      root,
      packRoot,
      profile,
      proofId,
      runId: args.runId ?? null,
    },
  );
  const capability =
    args.capability ?? definition.capabilities?.[0] ?? "local";
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
      status:
        capability === "manual-required" ||
        definition.collector === "manual-artifact"
          ? "manual-required"
          : "unavailable",
      message:
        "No executable command was provided; proof requires external/manual evidence.",
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
  return {
    ok: proofRun.status === "passed",
    proofRun,
    diagnostics: harnessReport.diagnostics,
  };
}

function proofStatus(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const runs = listProofRuns(root)
    .filter((run) => !args.proofId || run.proofId === args.proofId)
    .filter((run) => !args.status || run.status === args.status)
    .sort((left, right) =>
      String(right.startedAt).localeCompare(String(left.startedAt)),
    )
    .slice(0, args.limit ?? 20);
  return { ok: true, root, runs };
}

function proofLastFailure(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = listProofRuns(root)
    .filter((entry) => !args.proofId || entry.proofId === args.proofId)
    .sort((left, right) =>
      String(right.startedAt).localeCompare(String(left.startedAt)),
    )
    .find((entry) =>
      entry.status === "failed" ||
      entry.status === "manual-required" ||
      entry.status === "unavailable"
    );
  if (!run) return { ok: true, found: false, message: "No failed proof run found." };
  return {
    ok: true,
    found: true,
    proofRun: run,
    diagnostics: readProofDiagnostics(root, run.runId).slice(
      0,
      args.diagnosticLimit ?? args.limit ?? 10,
    ),
  };
}

function proofDiagnostics(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = args.runId
    ? readProofRun(root, args.runId)
    : latestProofRun(root, args.proofId);
  if (!run) return { ok: false, diagnostics: [], message: "No proof run found." };
  const diagnostics = readProofDiagnostics(root, run.runId).slice(
    0,
    args.limit ?? 50,
  );
  return { ok: true, runId: run.runId, proofId: run.proofId, diagnostics };
}

function proofArtifact(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const run = args.runId
    ? readProofRun(root, args.runId)
    : latestProofRun(root, args.proofId);
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
    run.artifacts.find(
      (entry) =>
        entry.kind === artifactName ||
        entry.kind === artifact ||
        entry.name === artifact,
    );
  if (!artifactRecord) {
    return { ok: false, text: "", message: `Unknown proof artifact: ${artifact}` };
  }
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

function proofReset(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const storage = proofStorageRoot(root);
  if (fs.existsSync(storage)) fs.rmSync(storage, { recursive: true, force: true });
  return { ok: true, root, removed: [PROOF_STORAGE_DIR] };
}

function proofPrune(input = {}) {
  const args = decodeProofQueryArguments(input);
  return pruneProofRuns({ root: path.resolve(args.root ?? process.cwd()) });
}

function proofExport(input = {}) {
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

function migrateLegacyProofs(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const profile = args.profile ?? projectProfileName(root);
  const scriptsRoot = path.join(root, args.scriptRoot ?? "scripts/test");
  const scripts = fs.existsSync(scriptsRoot)
    ? collectScriptFiles(scriptsRoot).map((filePath) =>
        classifyProofScript(root, filePath),
      )
    : [];
  const selectedScripts = scripts.filter((script) =>
    args.includeAllScripts
      ? true
      : script.name.includes("proof") ||
        script.claimSemantics.claimKinds.length > 0,
  );
  const generatedRegistry = {
    schemaVersion: 1,
    productName: `${profile} proof profile`,
    proofs: selectedScripts.map((script) =>
      legacyScriptProofDefinition({ profile, script }),
    ),
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
  fs.writeFileSync(
    targetRegistryPath,
    `${JSON.stringify(decoded, null, 2)}\n`,
    "utf8",
  );
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

function importLegacyProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadProofRegistry(packRoot, args.profile ?? "strict");
  const proofId = args.proofId ?? "PROOF-LEGACY-ARTIFACT-IMPORT";
  const definition =
    registry.proofs.find((proof) => proof.id === proofId) ??
    adHocProofDefinition(proofId, "file-hash");
  const legacy = collectLegacyProofArtifacts(root, args);
  const runId = args.runId ?? createProofRunId(proofId);
  const startedAt = new Date().toISOString();
  const git = gitState(root);
  const status =
    legacy.artifacts.length === 0 ||
    legacy.failedArtifacts.length > 0
      ? "failed"
      : "passed";
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
    claimsProved: uniqueSorted([
      ...(definition.claimsProved ?? []),
      ...legacy.claimsProved,
    ]),
    claimsNotProved: uniqueSorted([
      ...(definition.claimsNotProved ?? []),
      ...legacy.claimsNotProved,
    ]),
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
    const target = path.join(
      runDir,
      "artifacts",
      "legacy",
      sanitizeRelativeArtifactName(artifact.path),
    );
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
    {
      type: "proof-started",
      runId,
      proofId,
      timestamp: startedAt,
      source: "legacy-import",
    },
    {
      type: "legacy-artifacts-imported",
      runId,
      proofId,
      timestamp: startedAt,
      artifactCount: legacy.artifacts.length,
    },
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
  return {
    ok: proofRun.status === "passed",
    dryRun: false,
    root,
    proofRun,
    diagnostics,
  };
}

function proofParity(input = {}) {
  const args = decodeProofQueryArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const proofId = args.proofId ?? "PROOF-LEGACY-ARTIFACT-IMPORT";
  const legacy = collectLegacyProofArtifacts(root, args);
  const run = args.runId
    ? readProofRun(root, args.runId)
    : latestProofRun(root, proofId);
  const runLegacy = run?.legacy ?? null;
  const legacyHashes = new Map(
    legacy.artifacts.map((artifact) => [artifact.path, artifact.sha256]),
  );
  const importedHashes = new Map(
    (runLegacy?.artifacts ?? []).map((artifact) => [artifact.path, artifact.sha256]),
  );
  const missingInImported = [...legacyHashes]
    .filter(([artifactPath, hash]) => importedHashes.get(artifactPath) !== hash)
    .map(([artifactPath]) => artifactPath);
  const extraImported = [...importedHashes.keys()].filter(
    (artifactPath) => !legacyHashes.has(artifactPath),
  );
  const missingClaimsProved = legacy.claimsProved.filter(
    (claim) => !(run?.claimsProved ?? []).includes(claim),
  );
  const missingClaimsNotProved = legacy.claimsNotProved.filter(
    (claim) => !(run?.claimsNotProved ?? []).includes(claim),
  );
  const hasRun = Boolean(run);
  const comparable =
    legacy.artifacts.length > 0 && hasRun && runLegacy !== null;
  const equivalent =
    comparable &&
    missingInImported.length === 0 &&
    missingClaimsProved.length === 0 &&
    missingClaimsNotProved.length === 0;
  const coverage = equivalent
    ? "equivalent"
    : comparable
      ? "weaker"
      : "not-comparable";
  const deletionReady =
    equivalent &&
    run.status === "passed" &&
    legacy.failedArtifacts.length === 0;
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

function claimViolation(proofId, code, message) {
  return { proofId, code, message, severity: "error" };
}

function claimProof(input = {}, packRoot = DEFAULT_PACK_ROOT) {
  const args = decodeProofClaimArguments(input);
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadProofRegistry(packRoot);
  const proofIds = uniqueSorted(
    args.proofIds?.length ? args.proofIds : args.proofId ? [args.proofId] : [],
  );
  const ids =
    proofIds.length > 0
      ? proofIds
      : registry.proofs
          .filter((proof) => proof.requiredForPrReady)
          .map((proof) => proof.id);
  const currentGit = gitState(root);
  const violations = [];
  const accepted = [];

  for (const proofId of ids) {
    const definition =
      registry.proofs.find((proof) => proof.id === proofId) ?? null;
    const run = latestProofRun(root, proofId);
    if (!run) {
      violations.push(
        claimViolation(
          proofId,
          "missing-proof-run",
          "No proof run exists for this proof id.",
        ),
      );
      continue;
    }
    if (run.status !== "passed") {
      violations.push(
        claimViolation(
          proofId,
          "proof-not-passed",
          `Latest proof status is ${run.status}.`,
        ),
      );
    }
    if (currentGit.commit && run.git?.commit && currentGit.commit !== run.git.commit) {
      violations.push(
        claimViolation(
          proofId,
          "stale-commit",
          `Proof commit ${run.git.commit} does not match current commit ${currentGit.commit}.`,
        ),
      );
    }
    if (args.prReady && currentGit.dirty && !args.allowDirty) {
      violations.push(
        claimViolation(
          proofId,
          "dirty-worktree",
          "PR-ready proof claims require a clean worktree unless allowDirty is explicit.",
        ),
      );
    }
    for (const artifact of run.artifacts ?? []) {
      if (!fs.existsSync(path.join(root, artifact.path))) {
        violations.push(
          claimViolation(
            proofId,
            "missing-artifact",
            `Missing artifact ${artifact.path}.`,
          ),
        );
      }
    }
    for (const requiredPath of definition?.requiredPaths ?? []) {
      if (!fs.existsSync(repoAbsolute(root, requiredPath))) {
        violations.push(
          claimViolation(
            proofId,
            "deleted-required-path",
            `Required path is missing: ${requiredPath}.`,
          ),
        );
      }
    }
    accepted.push({
      proofId,
      runId: run.runId,
      status: run.status,
      commit: run.git?.commit ?? null,
    });
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

function runProofCli(tokens, options = {}) {
  const parsed = stripNullish(
    parseProofCli(tokens, options.defaultRoot ?? process.cwd()),
  );
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

export {
  loadProofRegistry,
  routeProofs,
  inventoryProofs,
  runProof,
  proofStatus,
  proofLastFailure,
  proofDiagnostics,
  proofArtifact,
  proofReset,
  proofPrune,
  proofExport,
  migrateLegacyProofs,
  importLegacyProof,
  proofParity,
  claimProof,
  runProofCli,
};
