import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

import {
  decodeCodexDoctorRequest,
  decodeCodexInstallRequest,
  decodeCodexUninstallRequest,
  decodeCheckReport,
  decodeCheckToolArguments,
  decodeCoordinationHealthReport,
  decodeCoordinationPresenceReport,
  decodeCoordinationToolArguments,
  decodeEnforcerConfig,
  decodeInitRequest,
  decodeProofClaimArguments,
  decodeProofClaimReport,
  decodeProofQueryArguments,
  decodeProofRegistry,
  decodeProofRouteRequest,
  decodeProofRunArguments,
  decodeProofRunReport,
  decodeRouteReport,
  decodeRouteRequest,
  decodeRuleRegistry,
  decodeRunReport,
  decodeRunToolArguments,
  decodeScanReport,
} from "../schemas/effect/enforcer-schemas.mjs";

const PACK_ROOT = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

test("Effect Schema decodes valid registry, config, route, init, and reports", () => {
  const registry = decodeRuleRegistry(
    JSON.parse(
      fs.readFileSync(path.join(PACK_ROOT, "rules", "rules.json"), "utf8"),
    ),
  );
  assert.equal(registry.productName, "ocentra-enforcer");
  assert.equal(registry.languages.includes("rust"), true);
  assert.equal(registry.languages.includes("typescript"), true);
  assert.equal(registry.languages.includes("python"), true);

  const proofRegistry = decodeProofRegistry(
    JSON.parse(
      fs.readFileSync(path.join(PACK_ROOT, "proof", "proofs.json"), "utf8"),
    ),
  );
  assert.equal(proofRegistry.productName, "ocentra-enforcer");
  assert.equal(
    proofRegistry.proofs.some((proof) => proof.id === "PROOF-COMMAND-GENERIC"),
    true,
  );

  const config = decodeEnforcerConfig(
    JSON.parse(
      fs.readFileSync(
        path.join(PACK_ROOT, "profiles", "ocentra-parent.json"),
        "utf8",
      ),
    ),
  );
  assert.equal(config.profileName, "ocentra-parent");

  const policyConfig = decodeEnforcerConfig({
    profileName: "docs-advisory",
    failOn: ["error"],
    importBoundaryPolicies: [
      { roots: ["apps/web"], forbiddenImports: ["@domain/*"] },
    ],
    architecturePolicyChecks: ["no-zod-source", "import-boundaries"],
    singleSourceRequiredMirrorRoots: ["crates/schema"],
    generatedArtifactsMode: "tracked",
    generatedArtifactsTracked: true,
    sourceShapePolicies: [
      { roots: ["src"], extensions: [".py"], kind: "python", maxFunctions: 10 },
    ],
    rules: {
      "DOC-1.1": { enabled: true, severity: "warning", note: "advisory only" },
      "TS-2.1": { severity: "error" },
    },
    tools: {
      cargoDoc: { enabled: false, severity: "warning" },
    },
  });
  assert.equal(policyConfig.rules["DOC-1.1"].severity, "warning");

  const route = decodeRouteRequest({
    root: PACK_ROOT,
    profile: "strict",
    scope: "files",
    files: ["src/lib.rs"],
  });
  assert.deepEqual(route.files, ["src/lib.rs"]);

  const init = decodeInitRequest({
    root: PACK_ROOT,
    profile: "strict",
    adapters: ["codex", "mcp", "precommit"],
    dryRun: true,
  });
  assert.equal(init.dryRun, true);

  const codexInstall = decodeCodexInstallRequest({
    root: PACK_ROOT,
    profile: "strict",
    dryRun: true,
    codexConfigPath: path.join(PACK_ROOT, "tmp", "config.toml"),
    ledgerRoot: path.join(PACK_ROOT, ".ledger"),
    serverName: "ocentra-enforcer",
    installSkill: true,
    installGlobalAgents: true,
  });
  assert.equal(codexInstall.serverName, "ocentra-enforcer");
  assert.equal(codexInstall.installGlobalAgents, true);
  assert.equal(codexInstall.ledgerRoot, path.join(PACK_ROOT, ".ledger"));

  const codexUninstall = decodeCodexUninstallRequest({
    dryRun: true,
    codexConfigPath: path.join(PACK_ROOT, "tmp", "config.toml"),
    serverName: "ocentra-enforcer",
    removeSkill: true,
    removeGlobalAgents: true,
  });
  assert.equal(codexUninstall.removeSkill, true);

  const codexDoctor = decodeCodexDoctorRequest({
    root: PACK_ROOT,
    codexConfigPath: path.join(PACK_ROOT, "tmp", "config.toml"),
    serverName: "ocentra-enforcer",
  });
  assert.equal(codexDoctor.serverName, "ocentra-enforcer");

  const routeReport = decodeRouteReport({
    ok: true,
    productName: "ocentra-enforcer",
    profileName: "strict",
    index: "rules/INDEX.md",
    scope: { mode: "files", files: ["src/lib.rs"] },
    docs: ["rules/rust/source.md#covered-rules"],
    rules: [
      {
        id: "RR-4.1",
        language: "rust",
        family: "source",
        severity: "error",
        doc: "rules/rust/source.md#covered-rules",
        validator: "rust/source-scan",
      },
    ],
  });
  assert.equal(routeReport.rules[0].id, "RR-4.1");

  const scanReport = decodeScanReport({
    ok: true,
    command: "scan",
    violations: [],
    warnings: [
      {
        ruleId: "DOC-1.1",
        severity: "warning",
        title: "Public API documentation is recommended",
        detail: "Exported API has no docs.",
        file: "src/lib.rs",
        line: 1,
        snippet: "Add a short doc comment.",
        doc: "rules/common/documentation.md#covered-rules",
        source: "pub fn thing() {}",
      },
    ],
    findings: [],
    bySeverity: { warning: 1 },
    failOn: ["error"],
    root: PACK_ROOT,
    profileName: "strict",
    scanOnly: true,
    scope: { mode: "files", files: ["src/lib.rs"] },
  });
  assert.equal(scanReport.ok, true);

  const runArgs = decodeRunToolArguments({
    root: PACK_ROOT,
    tool: "node",
    command: [process.execPath, "--version"],
  });
  assert.equal(runArgs.command[0], process.execPath);

  const proofRoute = decodeProofRouteRequest({
    root: PACK_ROOT,
    scope: "files",
    files: ["scripts/test/example-proof.mjs"],
    capability: "local",
  });
  assert.deepEqual(proofRoute.files, ["scripts/test/example-proof.mjs"]);

  const proofRunArgs = decodeProofRunArguments({
    root: PACK_ROOT,
    proofId: "PROOF-COMMAND-GENERIC",
    command: [process.execPath, "--version"],
    pin: true,
  });
  assert.equal(proofRunArgs.pin, true);

  const proofQuery = decodeProofQueryArguments({
    root: PACK_ROOT,
    proofId: "PROOF-COMMAND-GENERIC",
    status: "passed",
    limit: 5,
    includeScripts: true,
  });
  assert.equal(proofQuery.status, "passed");
  assert.equal(proofQuery.includeScripts, true);

  const proofClaimArgs = decodeProofClaimArguments({
    root: PACK_ROOT,
    proofIds: ["PROOF-COMMAND-GENERIC"],
    prReady: true,
  });
  assert.equal(proofClaimArgs.prReady, true);

  const checkArgs = decodeCheckToolArguments({
    root: PACK_ROOT,
    check: "source-shape",
    scope: "files",
    files: ["src/checks.mjs"],
    dryRun: true,
    staged: true,
    tracked: true,
    diagnosticLimit: 5,
    summaryOnly: true,
    groupBy: "file",
    includeScope: false,
  });
  assert.equal(checkArgs.check, "source-shape");
  assert.equal(checkArgs.tracked, true);
  assert.equal(checkArgs.summaryOnly, true);

  const coordinationArgs = decodeCoordinationToolArguments({
    stateRoot: path.join(PACK_ROOT, "tmp", "ledger"),
    hub: "generic-hub",
    lane: "codex-a",
    paths: ["src/lib.rs"],
    reason: "schema smoke",
    owner: "node_schema.codex-a",
    action: "add",
    peer: "left",
    url: "http://127.0.0.1:8787",
    mode: "pull",
    projectId: "schema-project",
    gitRemote: "git@example.com:ocentra/schema-project.git",
    branch: "feature/schema",
    commit: "abc1234",
    operation: "edit",
    lockKind: "writeLock",
    onConflict: "intent",
    claimGroup: "schema-contracts",
    codexThreadId: "thread-schema",
    limit: 5,
  });
  assert.equal(coordinationArgs.hub, "generic-hub");
  assert.equal(coordinationArgs.mode, "pull");
  assert.equal(coordinationArgs.owner, "node_schema.codex-a");
  assert.equal(coordinationArgs.operation, "edit");
  assert.equal(coordinationArgs.lockKind, "writeLock");
  assert.equal(coordinationArgs.onConflict, "intent");
  assert.equal(coordinationArgs.branch, "feature/schema");

  const coordinationHealth = decodeCoordinationHealthReport({
    ok: true,
    root: path.join(PACK_ROOT, "tmp", "ledger"),
    canInspect: true,
    canLockPaths: true,
    canWriteClaimedPaths: true,
    mustWait: false,
    mustRepairLedger: false,
    diagnostics: [],
    warnings: [],
    conflicts: [],
    hardConflicts: [],
    branchWriteConflicts: [],
    mergeRisks: [],
    globalWriteConflicts: [],
    editIntents: [],
    staleSessions: [],
    guard: null,
    dashboard: {},
  });
  assert.equal(coordinationHealth.mustWait, false);

  const coordinationPresence = decodeCoordinationPresenceReport({
    ok: true,
    root: path.join(PACK_ROOT, "tmp", "ledger"),
    generatedAt: "2026-01-01T00:00:00.000Z",
    totalRows: 1,
    rows: [{ lane: "codex-a", projectId: "schema-project" }],
    views: { byLane: { "codex-a": [] } },
  });
  assert.equal(coordinationPresence.totalRows, 1);

  const checkReport = decodeCheckReport({
    ok: true,
    command: "check",
    check: "architecture-policy",
    root: PACK_ROOT,
    profileName: "strict",
    violations: [],
    warnings: [],
    findings: [],
    bySeverity: {},
    scope: { mode: "files", files: ["src/checks.mjs"] },
    checks: [{ check: "no-zod-source", ok: true, violations: 0 }],
  });
  assert.equal(checkReport.ok, true);
  assert.equal(checkReport.checks[0].check, "no-zod-source");

  const runReport = decodeRunReport({
    ok: true,
    summary: {
      runId: "run-1",
      root: PACK_ROOT,
      profile: "strict",
      tool: "node",
      language: "common",
      cwd: ".",
      command: [process.execPath, "--version"],
      status: "passed",
      exitCode: 0,
      startedAt: "2026-01-01T00:00:00.000Z",
      endedAt: "2026-01-01T00:00:01.000Z",
      diagnosticCount: 0,
      bySeverity: {},
      artifacts: {
        stdout: ".enforce/runs/run-1/raw/stdout.log",
        stderr: ".enforce/runs/run-1/raw/stderr.log",
      },
      duckdb: { available: false },
    },
    diagnostics: [],
  });
  assert.equal(runReport.summary.status, "passed");

  const proofRunReport = decodeProofRunReport({
    ok: true,
    proofRun: {
      schemaVersion: 1,
      proofId: "PROOF-COMMAND-GENERIC",
      title: "Generic command proof with bounded artifacts",
      family: "command",
      collector: "command",
      profile: "strict",
      root: PACK_ROOT,
      runId: "proof-run-1",
      status: "passed",
      ok: true,
      exitCode: 0,
      startedAt: "2026-01-01T00:00:00.000Z",
      endedAt: "2026-01-01T00:00:01.000Z",
      command: [process.execPath, "--version"],
      diagnosticCount: 0,
      pinned: false,
      git: { commit: null },
      scope: { capability: "local" },
      claimsProved: [],
      claimsNotProved: [],
      retention: {
        maxRunsPerProof: 20,
        maxFailedRuns: 20,
        maxArtifactBytes: 52428800,
        pruneAfterDays: 14,
        pinPrReadyDays: 30,
      },
      artifacts: [
        {
          name: "proof-run.json",
          kind: "proof-run",
          path: ".enforce/proofs/runs/proof-run-1/proof-run.json",
          sha256: "abc",
          byteLength: 3,
        },
      ],
    },
    diagnostics: [],
  });
  assert.equal(proofRunReport.proofRun.status, "passed");

  const proofClaimReport = decodeProofClaimReport({
    ok: true,
    root: PACK_ROOT,
    claim: {
      schemaVersion: 1,
      claimId: "claim-1",
      proofIds: ["PROOF-COMMAND-GENERIC"],
      checkedAt: "2026-01-01T00:00:00.000Z",
      violations: [],
    },
  });
  assert.equal(proofClaimReport.ok, true);

  for (const schemaName of [
    "proof-capability.schema.json",
    "proof-retention-policy.schema.json",
    "proof-definition.schema.json",
    "proof-artifact.schema.json",
    "proof-diagnostic.schema.json",
    "proof-run.schema.json",
    "proof-claim.schema.json",
    "proof-tool-arguments.schema.json",
  ]) {
    assert.equal(
      fs.existsSync(path.join(PACK_ROOT, "schemas", "json", schemaName)),
      true,
      `missing ${schemaName}`,
    );
  }
});

test("Effect Schema rejects invalid external payloads with useful labels", () => {
  assert.throws(
    () => decodeRouteRequest({ files: "src/lib.rs" }),
    /route request schema validation failed/u,
  );
  assert.throws(
    () => decodeInitRequest({ adapters: ["husky", "unknown"] }),
    /init request schema validation failed/u,
  );
  assert.throws(
    () => decodeCodexInstallRequest({ dryRun: "yes" }),
    /codex install request schema validation failed/u,
  );
  assert.throws(
    () => decodeCodexUninstallRequest({ removeSkill: "yes" }),
    /codex uninstall request schema validation failed/u,
  );
  assert.throws(
    () => decodeCodexDoctorRequest({ serverName: 42 }),
    /codex doctor request schema validation failed/u,
  );
  assert.throws(
    () => decodeCheckToolArguments({ check: "not-real" }),
    /check tool arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeRunToolArguments({ command: "node --version" }),
    /run tool arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeProofRouteRequest({ capability: "telepathy" }),
    /proof route request schema validation failed/u,
  );
  assert.throws(
    () => decodeProofRunArguments({ command: "node --version" }),
    /proof run arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeProofClaimArguments({ proofIds: "PROOF-COMMAND-GENERIC" }),
    /proof claim arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeCoordinationToolArguments({ paths: "src/lib.rs" }),
    /coordination tool arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeCoordinationToolArguments({ operation: "write" }),
    /coordination tool arguments schema validation failed/u,
  );
  assert.throws(
    () => decodeCoordinationToolArguments({ lockKind: "global" }),
    /coordination tool arguments schema validation failed/u,
  );
  assert.throws(
    () =>
      decodeRuleRegistry({
        schemaVersion: 1,
        productName: "ocentra-enforcer",
        languages: ["rust"],
        rules: [{ id: "RR-1.1", language: "rust", family: "not-real" }],
      }),
    /rule registry schema validation failed/u,
  );
});

test("JSON-schema-compatible artifacts are present for non-Effect consumers", () => {
  const schemas = [
    [
      "schemas/json/enforcer-config.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/enforcer-config.schema.json",
    ],
    [
      "schemas/json/codex-install-request.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/codex-install-request.schema.json",
    ],
    [
      "schemas/json/codex-doctor-request.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/codex-doctor-request.schema.json",
    ],
    [
      "schemas/json/codex-uninstall-request.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/codex-uninstall-request.schema.json",
    ],
    [
      "schemas/json/check-tool-arguments.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/check-tool-arguments.schema.json",
    ],
    [
      "schemas/json/check-report.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/check-report.schema.json",
    ],
    [
      "schemas/json/route-request.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/route-request.schema.json",
    ],
    [
      "schemas/json/rule-registry.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/rule-registry.schema.json",
    ],
    [
      "schemas/json/diagnostic.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/diagnostic.schema.json",
    ],
    [
      "schemas/json/run-report.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/run-report.schema.json",
    ],
    [
      "schemas/json/run-tool-arguments.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/run-tool-arguments.schema.json",
    ],
    [
      "schemas/json/coordination-tool-arguments.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/coordination-tool-arguments.schema.json",
    ],
    [
      "schemas/json/coordination-health-report.schema.json",
      "https://ocentra.dev/schemas/ocentra-enforcer/coordination-health-report.schema.json",
    ],
  ];

  for (const [relPath, id] of schemas) {
    const parsed = JSON.parse(
      fs.readFileSync(path.join(PACK_ROOT, relPath), "utf8"),
    );
    assert.equal(
      parsed.$schema,
      "https://json-schema.org/draft/2020-12/schema",
    );
    assert.equal(parsed.$id, id);
    assert.equal(parsed.title.startsWith("Ocentra Enforcer"), true);
  }
});
