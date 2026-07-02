#!/usr/bin/env node
import {
  CLI_PATH,
  COMMON_INPUT_SCHEMA,
  COMPACT_RESULT_SCHEMA,
  COORDINATION_WRITE_TOOLS,
  MCP_PROTOCOL_VERSION,
  PACKAGE_JSON,
  RULE_REGISTRY_PATH,
  SERVER_ROOT,
  SERVER_STARTED_AT,
  SCOPE_SCHEMA,
} from "./rust-rules-mcp-context.mjs";
import {
  buildMcpFingerprint,
  changedFingerprintFiles,
  commonCoordinationOptions,
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
  extraFingerprintFiles,
  fingerprintFile,
  mcpStaleError,
  normalizeFingerprintLabel,
  normalizeToolName,
  pathOption,
  pushOption,
  quoteCommandArg,
  readPackageVersion,
  reasonOption,
  resolveFingerprintFile,
  shouldBlockStaleMcpTool,
  stringArray,
  summarizeFingerprintEntry,
} from "./rust-rules-mcp-freshness.mjs";
import {
  compactFinding,
  compactScope,
  countBy,
  groupFindings,
  maybeCompactReport,
  parseJson,
  uniqueSorted,
} from "./rust-rules-mcp-compact.mjs";

const MCP_FINGERPRINT_FILES = [
  "package.json",
  "mcp/rust-rules-mcp.mjs",
  "schemas/effect/enforcer-schemas.mjs",
  "scripts/rust-rules.mjs",
  "src/coordination/api.mjs",
  "src/coordination/runner.mjs",
  "src/coordination/vendor/events.js",
  "src/coordination/vendor/repair.js",
  "src/coordination/vendor/stream.js",
  ...extraFingerprintFiles(),
];

function runQueryInputSchema() {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      root: COMMON_INPUT_SCHEMA.properties.root,
      runId: {
        type: "string",
        description: "Optional run id. Defaults to the latest run.",
      },
      limit: {
        type: "number",
        description: "Maximum run or diagnostic rows to return.",
      },
      diagnosticLimit: {
        type: "number",
        description: "Maximum diagnostics for last-failure.",
      },
      severity: { type: "string", enum: ["error", "warning", "info"] },
      status: {
        type: "string",
        enum: ["passed", "failed"],
        description: "Optional run status filter.",
      },
      file: {
        type: "string",
        description: "Optional file filter for diagnostics.",
      },
      tool: { type: "string", description: "Optional logical tool filter." },
      crateName: {
        type: "string",
        description: "Optional Cargo crate/package metadata filter.",
      },
      packageName: {
        type: "string",
        description: "Optional JS/Python package metadata filter.",
      },
      domain: {
        type: "string",
        description: "Optional domain metadata filter.",
      },
      tag: { type: "string", description: "Optional run tag filter." },
      artifact: {
        type: "string",
        enum: ["stdout", "stderr", "diagnostics", "events"],
      },
      limitBytes: {
        type: "number",
        description: "Maximum artifact bytes to return.",
      },
    },
  };
}

function coordinationInputSchema(extra = {}) {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      root: {
        type: "string",
        description: "Target repository root for future adapter-aware checks.",
      },
      stateRoot: {
        type: "string",
        description:
          "Exact coordination hub root for repair/import overrides. Defaults to OCENTRA_LEDGER_HOME/<hub>.",
      },
      hub: {
        type: "string",
        description: "Generic hub name used when stateRoot is not explicit.",
      },
      lane: { type: "string", description: "Lane id." },
      paths: {
        type: "array",
        items: { type: "string" },
        description: "Exact changed or claimed paths.",
      },
      changedPaths: {
        type: "array",
        items: { type: "string" },
        description: "Changed paths for guard/health decisions.",
      },
      reason: { type: "string" },
      summary: { type: "string" },
      owner: {
        type: "string",
        description:
          "Optional writer id to preserve for coordination repair stale-claims.",
      },
      operation: {
        type: "string",
        enum: ["inspect", "edit", "commit", "push", "rebase", "merge", "pr_ready"],
        description:
          "Operation-aware guard mode. Defaults to commit for guard/health with paths and edit for claim.",
      },
      lockKind: {
        type: "string",
        enum: ["writeLock", "globalWriteLock", "branchLease", "workReservation"],
        description:
          "Claim kind. Normal exact-file writes use writeLock; singleton paths promote to globalWriteLock.",
      },
      onConflict: {
        type: "string",
        enum: ["fail", "intent"],
        description:
          "When claim is blocked, fail immediately or append an editIntent queue record.",
      },
      claimGroup: {
        type: "string",
        description:
          "Optional group key so source/generated files or related paths lock together.",
      },
      waitMs: {
        type: "number",
        description:
          "Reserved wait budget for future polling; current v1 records intent instead of blocking.",
      },
      from: {
        type: "string",
        description:
          "Optional sender lane for coordination messages. Defaults to the hub identity default lane.",
      },
      to: { type: "string", description: "Message recipient lane/address." },
      subject: {
        type: "string",
        description:
          "Optional message subject. The wire event stores subject as a body prefix for compatibility.",
      },
      body: { type: "string" },
      message: { type: "string", description: "Alias for body." },
      messageId: { type: "string" },
      taskId: { type: "string" },
      state: { type: "string" },
      sessionId: { type: "string" },
      action: {
        type: "string",
        description:
          "Coordination action for aggregate tools such as peer or mail.",
      },
      peer: { type: "string" },
      peerUrl: { type: "string" },
      url: { type: "string" },
      name: { type: "string" },
      token: { type: "string" },
      tokenEnv: { type: "string" },
      mode: { type: "string", enum: ["pull", "push", "both"] },
      host: { type: "string" },
      port: { type: "number" },
      keepLatest: { type: "number" },
      projectId: { type: "string" },
      repoRoot: { type: "string" },
      worktreeRoot: { type: "string" },
      cwd: { type: "string" },
      gitRemote: { type: "string" },
      branch: { type: "string" },
      commit: { type: "string" },
      codexThreadId: { type: "string" },
      codexSessionId: { type: "string" },
      stateFile: { type: "string" },
      peek: { type: "boolean" },
      dryRun: { type: "boolean" },
      write: { type: "boolean" },
      focused: { type: "boolean" },
      allowPrimaryWithoutClaims: { type: "boolean" },
      allowMergeRisks: { type: "boolean" },
      all: { type: "boolean" },
      allOwned: { type: "boolean" },
      allLanes: { type: "boolean" },
      allowOtherNode: { type: "boolean" },
      releaseOwned: { type: "boolean" },
      repairStale: { type: "boolean" },
      limit: { type: "number" },
      ...extra,
    },
  };
}

function coordinationActionInputSchema(action, description) {
  const actions = Array.isArray(action) ? action : [action];
  return coordinationInputSchema({
    action: {
      type: "string",
      enum: actions,
      description,
    },
  });
}

function proofInputSchema(extra = {}) {
  return {
    type: "object",
    additionalProperties: false,
    properties: {
      root: COMMON_INPUT_SCHEMA.properties.root,
      profile: COMMON_INPUT_SCHEMA.properties.profile,
      scope: SCOPE_SCHEMA,
      files: COMMON_INPUT_SCHEMA.properties.files,
      plan: { type: "string" },
      capability: {
        type: "string",
        enum: [
          "ci",
          "local",
          "windows",
          "linux",
          "macos",
          "wsl",
          "android-emulator",
          "android-device",
          "ios-simulator",
          "ios-device",
          "browser",
          "network",
          "cloud",
          "manual-required",
        ],
      },
      proofId: { type: "string" },
      proofIds: { type: "array", items: { type: "string" } },
      runId: { type: "string" },
      command: { type: "array", items: { type: "string" } },
      tags: { type: "array", items: { type: "string" } },
      artifact: { type: "string" },
      legacyPaths: {
        type: "array",
        items: { type: "string" },
        description:
          "Legacy proof artifact files or directories to import or compare. Defaults to test-results, output, and docs/proof.",
      },
      limit: { type: "number" },
      diagnosticLimit: { type: "number" },
      limitBytes: { type: "number" },
      includeScripts: {
        type: "boolean",
        description:
          "For proof inventory only: include bounded script rows. Defaults to false.",
      },
      status: {
        type: "string",
        enum: ["passed", "failed", "manual-required", "unavailable", "waived"],
      },
      pin: { type: "boolean" },
      claimId: { type: "string" },
      prReady: { type: "boolean" },
      allowDirty: { type: "boolean" },
      dryRun: { type: "boolean" },
      ...extra,
    },
  };
}

export {
  CLI_PATH,
  COMMON_INPUT_SCHEMA,
  COMPACT_RESULT_SCHEMA,
  COORDINATION_WRITE_TOOLS,
  MCP_FINGERPRINT_FILES,
  MCP_PROTOCOL_VERSION,
  PACKAGE_JSON,
  RULE_REGISTRY_PATH,
  SERVER_ROOT,
  SERVER_STARTED_AT,
  SCOPE_SCHEMA,
  buildMcpFingerprint,
  changedFingerprintFiles,
  coordinationActionInputSchema,
  coordinationCommandFallbackArgs,
  coordinationFallbackArgs,
  coordinationFallbackCommand,
  coordinationGlobalFallbackArgs,
  coordinationInputSchema,
  commonCoordinationOptions,
  extraFingerprintFiles,
  compactFinding,
  compactScope,
  countBy,
  groupFindings,
  fingerprintFile,
  mcpStaleError,
  maybeCompactReport,
  parseJson,
  normalizeFingerprintLabel,
  normalizeToolName,
  pathOption,
  proofInputSchema,
  pushOption,
  quoteCommandArg,
  reasonOption,
  readPackageVersion,
  resolveFingerprintFile,
  runQueryInputSchema,
  shouldBlockStaleMcpTool,
  stringArray,
  summarizeFingerprintEntry,
  uniqueSorted,
};
