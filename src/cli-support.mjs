import fs from "node:fs";

const DEFAULT_RULE_DOC = "docs/RustRules.md";
const FINDING_COLLECTIONS = ["violations", "warnings", "waived", "findings"];

export function createRuleDocFor({ ruleRegistryPath, decodeRuleRegistry }) {
  let cachedRuleDocs = null;
  return function ruleDocFor(ruleId) {
    cachedRuleDocs ??= loadRuleDocs(ruleRegistryPath, decodeRuleRegistry);
    return (
      cachedRuleDocs.get(ruleId) ??
      `${DEFAULT_RULE_DOC}#${String(ruleId).toLowerCase().replace(".", "")}`
    );
  };
}

function loadRuleDocs(ruleRegistryPath, decodeRuleRegistry) {
  const ruleDocs = new Map();
  if (!fs.existsSync(ruleRegistryPath)) return ruleDocs;
  const registry = decodeRuleRegistry(
    JSON.parse(fs.readFileSync(ruleRegistryPath, "utf8")),
  );
  for (const entry of registry.rules ?? []) {
    ruleDocs.set(entry.id, entry.doc);
  }
  return ruleDocs;
}

export function createExplainRule({
  rulesById,
  genericRules,
  checkRules,
  ruleDocFor,
}) {
  return function explainRule(ruleId) {
    const normalized = ruleId?.toUpperCase();
    const rule =
      rulesById[normalized] ??
      genericRules[normalized] ??
      checkRules[normalized];
    if (!rule) throw new Error(`Unknown rule ID: ${ruleId}`);
    return { ruleId: normalized, ...rule, anchor: ruleDocFor(normalized) };
  };
}

export function createDoctor({ commandExists }) {
  return function doctor(root, config, scope) {
    const checks = [
      { name: "root", ok: fs.existsSync(root), detail: root },
      {
        name: "config schema",
        ok: config.schemaVersion >= 1,
        detail: `schemaVersion=${config.schemaVersion}`,
      },
      {
        name: "cargo",
        ok: commandExists("cargo"),
        detail: "required for cargo gates and metadata dependency checks",
      },
      {
        name: "git",
        ok: commandExists("git"),
        detail: "required for diff scopes",
      },
      {
        name: "cargo-deny",
        ok: !config.requireCargoDeny || commandExists("cargo-deny"),
        detail: config.requireCargoDeny
          ? "required when requireCargoDeny=true"
          : "not required by this profile",
      },
      {
        name: "scope files",
        ok: scope.files.length > 0,
        detail: `${scope.files.length} Rust file(s) selected`,
      },
    ];
    return {
      ok: checks.every((check) => check.ok),
      command: "doctor",
      root,
      profileName: config.profileName,
      scope,
      checks,
      violations: [],
    };
  };
}

export function printDoctor(report) {
  for (const check of report.checks) {
    console.log(`${check.ok ? "PASS" : "FAIL"} ${check.name}: ${check.detail}`);
  }
}

const RUNS_COMMANDS = {
  list(query, ops) {
    return { ok: true, runs: ops.listRuns(query) };
  },
  summary(query, ops) {
    return { ok: true, summary: ops.runSummary(query) };
  },
  diagnostics(query, ops) {
    return ops.runDiagnostics(query);
  },
  "last-failure"(query, ops) {
    return ops.lastFailure(query);
  },
  triage(query, ops) {
    return ops.triageCiLog(query);
  },
  artifact(query, ops) {
    return ops.readArtifact(query);
  },
  prune(query, ops) {
    return ops.pruneRuns(query);
  },
  reset(query, ops) {
    return ops.resetRuns(query);
  },
  ingest() {
    return {
      ok: true,
      message:
        "NDJSON manifests are updated at run completion; DuckDB ingestion is optional in this build.",
    };
  },
};

export function runRunsCommand(args, root, config, ops) {
  const query = {
    root,
    harness: config.harness,
    runId: args.runId,
    limit: args.limit ?? undefined,
    diagnosticLimit: args.limit ?? undefined,
    severity: args.severity ?? undefined,
    status: args.status ?? undefined,
    file: args.file ?? undefined,
    tool: args.runTool ?? undefined,
    crateName: args.crateName ?? undefined,
    packageName: args.packageName ?? undefined,
    domain: args.domain ?? undefined,
    tag: args.tag ?? undefined,
    artifact: args.artifact ?? undefined,
    limitBytes: args.limitBytes ?? undefined,
  };
  const handler = RUNS_COMMANDS[args.runsCommand];
  if (!handler) throw new Error(`Unknown runs command: ${args.runsCommand}`);
  return handler(query, ops);
}

export { FINDING_COLLECTIONS };
