#!/usr/bin/env node
/*
 * MCP stdio adapter for Ocentra Enforcer.
 * rust-rules-mcp remains a compatibility filename for one release.
 */
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { fileURLToPath } from "node:url";
import {
  decodeCheckToolArguments,
  decodeCoordinationToolArguments,
  decodeDoctorToolArguments,
  decodeExplainToolArguments,
  decodeRouteRequest,
  decodeRuleRegistry,
  decodeRunQueryArguments,
  decodeRunToolArguments,
  decodeScanToolArguments,
} from "../schemas/effect/enforcer-schemas.mjs";
import {
  coordinationAck,
  coordinationClaim,
  coordinationCloseout,
  coordinationGuard,
  coordinationHealth,
  coordinationIndex,
  coordinationInbox,
  coordinationInit,
  coordinationMail,
  coordinationMessage,
  coordinationNotify,
  coordinationPeer,
  coordinationPresence,
  coordinationRelease,
  coordinationRepair,
  coordinationReport,
  coordinationCompact,
  coordinationEnsure,
  coordinationStreams,
  coordinationSync,
  coordinationStatus,
  coordinationTasks,
  coordinationWorkers,
} from "../src/coordination/api.mjs";
import { coordinationHashCompatibility } from "../src/coordination/vendor/events.js";
import { routeRules as buildRouteReport } from "../src/routing.mjs";
import {
  lastFailure,
  listRuns,
  pruneRuns,
  readArtifact,
  resetRuns,
  runDiagnostics,
  runHarness,
  runSummary,
} from "../src/harness.mjs";
import {
  claimProof,
  importLegacyProof,
  inventoryProofs,
  proofArtifact,
  proofDiagnostics,
  proofExport,
  proofLastFailure,
  proofParity,
  proofPrune,
  proofReset,
  proofStatus,
  routeProofs,
  runProof,
} from "../src/proof.mjs";

const MCP_PROTOCOL_VERSION = "2025-06-18";
const SERVER_ROOT = path.resolve(
  path.join(path.dirname(fileURLToPath(import.meta.url)), ".."),
);
const CLI_PATH = path.join(SERVER_ROOT, "scripts", "rust-rules.mjs");
const RULE_REGISTRY_PATH = path.join(SERVER_ROOT, "rules", "rules.json");
const PACKAGE_JSON = JSON.parse(
  fs.readFileSync(path.join(SERVER_ROOT, "package.json"), "utf8"),
);
const SERVER_STARTED_AT = new Date().toISOString();
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
const STARTUP_FINGERPRINT = buildMcpFingerprint();
const COORDINATION_WRITE_TOOLS = new Set([
  "ocentra_enforcer_coordination_init",
  "ocentra_enforcer_coordination_claim",
  "ocentra_enforcer_coordination_closeout",
  "ocentra_enforcer_coordination_release",
  "ocentra_enforcer_coordination_report",
  "ocentra_enforcer_coordination_message",
  "ocentra_enforcer_coordination_sync",
  "ocentra_enforcer_coordination_ensure",
  "ocentra_enforcer_coordination_compact",
]);

const SCOPE_SCHEMA = {
  type: "string",
  enum: ["workspace", "files", "crate", "diff"],
  description:
    "Validation scope. Defaults to workspace unless files, crateName, or base/head imply a narrower scope.",
};

const COMMON_INPUT_SCHEMA = {
  type: "object",
  additionalProperties: false,
  properties: {
    root: {
      type: "string",
      description:
        "Target repository root. Defaults to the MCP server working directory.",
    },
    configPath: {
      type: "string",
      description:
        "Optional Ocentra Enforcer config path. Relative paths resolve against root.",
    },
    profile: {
      type: "string",
      description:
        "Optional named pack profile such as strict or ocentra-parent. Ignored when configPath is provided.",
    },
    scope: SCOPE_SCHEMA,
    files: {
      type: "array",
      items: { type: "string" },
      description: "Files or directories for files scope.",
    },
    crateName: {
      type: "string",
      description: "Cargo package name for crate scope.",
    },
    languages: {
      type: "array",
      items: {
        type: "string",
        enum: ["rust", "typescript", "python", "common"],
      },
      description:
        "Optional scan languages. Defaults to the target config/profile.",
    },
    base: {
      type: "string",
      description: "Base git ref for diff scope.",
    },
    head: {
      type: "string",
      description: "Head git ref for diff scope.",
    },
  },
};

const COMPACT_RESULT_SCHEMA = {
  diagnosticLimit: {
    type: "number",
    description: "Maximum findings to include in compact MCP output.",
  },
  summaryOnly: {
    type: "boolean",
    description:
      "Return only summary/group counts without individual findings.",
  },
  groupBy: {
    type: "string",
    enum: ["file", "slice"],
    description: "Group compact findings by file or top-level repo slice.",
  },
  includeScope: {
    type: "boolean",
    description: "When false, omit full scope file lists from MCP output.",
  },
};

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

const CANONICAL_TOOLS = [
  {
    name: "ocentra_enforcer_mcp_status",
    description:
      "Report MCP process freshness and whether Codex must reload this Enforcer server before coordination writes.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      properties: {},
    },
  },
  {
    name: "ocentra_enforcer_route",
    description:
      "Return compact indexed Ocentra Enforcer rule docs relevant to files, crate, scope, profile, or one rule ID.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        ruleId: {
          type: "string",
          description:
            "Optional explicit rule ID such as RR-7.3. When provided, routes directly to that rule.",
        },
      },
    },
  },
  {
    name: "ocentra_enforcer_scan",
    description:
      "Run deterministic Ocentra Enforcer scanner by workspace, files, crate, or diff scope.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        cargo: {
          type: "boolean",
          description:
            "When true, run cargo gates in addition to scanner checks.",
          default: false,
        },
        ...COMPACT_RESULT_SCHEMA,
      },
    },
  },
  {
    name: "ocentra_enforcer_check",
    description:
      "Run a named Ocentra Enforcer reusable check such as no-zod-source, source-shape, dependency-policy, or sbom.",
    inputSchema: {
      ...COMMON_INPUT_SCHEMA,
      required: ["check"],
      properties: {
        ...COMMON_INPUT_SCHEMA.properties,
        check: {
          type: "string",
          enum: [
            "no-zod-source",
            "no-naked-domain-strings",
            "no-test-doubles",
            "weak-assertions",
            "skipped-focused-tests",
            "validation-bypass",
            "placeholder-implementation",
            "reexports",
            "cross-platform-script-commands",
            "generated-artifacts",
            "secrets",
            "rust-string-boundaries",
            "source-shape",
            "required-tests",
            "single-source-contracts",
            "dependency-policy",
            "sbom",
            "ai-rule-index",
            "import-boundaries",
            "architecture-policy",
          ],
          description: "Named reusable check to run.",
        },
        checkConfigPath: {
          type: "string",
          description:
            "Optional check-specific config path, for example a single-source contract config.",
        },
        output: {
          type: "string",
          description: "Optional output directory for checks such as sbom.",
        },
        dryRun: {
          type: "boolean",
          description:
            "Validate the check path without writing generated outputs where supported.",
        },
        staged: {
          type: "boolean",
          description: "With check secrets: scan staged files only.",
        },
        tracked: {
          type: "boolean",
          description:
            "With check generated-artifacts: include tracked generated paths.",
        },
        strictEmptyTestTrees: {
          type: "boolean",
          description:
            "With check required-tests: reject tests/proof trees that only contain .gitkeep.",
        },
        ...COMPACT_RESULT_SCHEMA,
      },
    },
  },
  {
    name: "ocentra_enforcer_run",
    description:
      "Run a command through the Enforcer harness, persist raw logs, emit NDJSON diagnostics, and return a compact summary.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      required: ["command"],
      properties: {
        root: COMMON_INPUT_SCHEMA.properties.root,
        profile: COMMON_INPUT_SCHEMA.properties.profile,
        tool: {
          type: "string",
          description:
            "Logical tool name such as cargo-check, eslint, pytest, or tsc.",
        },
        language: {
          type: "string",
          enum: ["rust", "typescript", "python", "common"],
        },
        cwd: {
          type: "string",
          description: "Optional working directory relative to root.",
        },
        runId: {
          type: "string",
          description: "Optional caller-provided run id.",
        },
        crateName: {
          type: "string",
          description: "Optional Cargo crate/package metadata.",
        },
        packageName: {
          type: "string",
          description: "Optional JS/Python package metadata.",
        },
        domain: { type: "string", description: "Optional domain metadata." },
        command: {
          type: "array",
          items: { type: "string" },
          description: "Executable and arguments.",
        },
        tags: { type: "array", items: { type: "string" } },
      },
    },
  },
  {
    name: "ocentra_enforcer_proof_route",
    description:
      "Return compact indexed proof definitions relevant to files, plan, capability, profile, or one proof id.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_run",
    description:
      "Run or record a proof through the Enforcer proof harness, storing local proof artifacts under .enforce/proofs.",
    inputSchema: proofInputSchema({
      proofId: { type: "string" },
      command: { type: "array", items: { type: "string" } },
    }),
  },
  {
    name: "ocentra_enforcer_proof_status",
    description: "Return compact proof run status for the target repository.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_inventory",
    description:
      "Read-only inventory of legacy proof scripts in a target repository, grouped by family/capability.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_import_legacy",
    description:
      "Read legacy proof artifacts and write canonical Enforcer proof runs under .enforce/proofs.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_parity",
    description:
      "Compare legacy proof artifacts with an imported Enforcer proof run and report deletion readiness.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_claim",
    description:
      "Validate that named proof ids support a PR-ready or completion claim without stale/missing artifacts.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_last_failure",
    description: "Return the latest failed/manual-required proof with compact diagnostics.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_diagnostics",
    description: "Return compact proof diagnostics for the latest or requested proof run.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_artifact",
    description: "Return a bounded proof artifact only when compact diagnostics are insufficient.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_reset",
    description: "Delete local proof run state under .enforce/proofs for a target repository.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_prune",
    description: "Apply proof retention policy under .enforce/proofs.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_proof_export",
    description:
      "Return a manifest-only proof export suitable for CI artifact upload metadata.",
    inputSchema: proofInputSchema(),
  },
  {
    name: "ocentra_enforcer_run_status",
    description: "Return the latest or requested Enforcer harness run summary.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_diagnostics",
    description:
      "Return compact diagnostics for the latest or requested harness run.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_last_failure",
    description:
      "Return the latest failed harness run with compact diagnostics.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_artifact",
    description:
      "Return a bounded raw harness artifact only when compact diagnostics are insufficient.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_prune_runs",
    description:
      "Apply target repo harness retention policy without deleting the whole store.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_reset_runs",
    description: "Delete harness run artifacts for a target root.",
    inputSchema: runQueryInputSchema(),
  },
  {
    name: "ocentra_enforcer_doctor",
    description:
      "Check Ocentra Enforcer wiring for a target root/config/scope without changing files.",
    inputSchema: COMMON_INPUT_SCHEMA,
  },
  {
    name: "ocentra_enforcer_coordination_init",
    description: "Initialize generic external coordination state for a hub/lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_health",
    description:
      "Return compact generic hub/lane/mail/worktree coordination health and write-safety decisions.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_presence",
    description:
      "Return compact PC/project/worktree/lane/thread/claim presence matrix for the coordination hub.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_index",
    description:
      "Rebuild disposable coordination read indexes and JSON views from canonical streams.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_streams",
    description:
      "Return stream manifest with event counts, byte lengths, seq ranges, and tail hashes.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_sync",
    description:
      "Sync coordination streams from a local or HTTP peer using manifest plus suffix transfer.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_peer",
    description:
      "Manage and inspect coordination peers: add, remove, list, health, status, or sync.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_ensure",
    description:
      "Ensure the background coordination peer daemon is running for this state root.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_compact",
    description:
      "Compact hot streams into immutable archive segments and rebuild read indexes.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_notify",
    description: "Return wake/notification requests for a coordination lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_mail",
    description: "Aggregate mail helper for inbox, send, and ack actions.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_status",
    description: "Return materialized generic coordination state.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_inbox",
    description: "Return unread or all messages for a lane.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_claim",
    description:
      "Claim exact paths for a lane. Use ocentra_enforcer_coordination_release to release paths.",
    inputSchema: coordinationActionInputSchema(
      "claim",
      "Optional dedicated-tool action marker. action=\"release\" is invalid here.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_release",
    description:
      "Release exact paths for a lane. Use ocentra_enforcer_coordination_claim to claim paths.",
    inputSchema: coordinationActionInputSchema(
      "release",
      "Optional dedicated-tool action marker. action=\"claim\" is invalid here.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_closeout",
    description:
      "Release and stale-repair all claims for the selected lane/thread scope, rebuild indexes, and fail if any claims remain.",
    inputSchema: coordinationActionInputSchema(
      "closeout",
      "Optional dedicated-tool action marker.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_repair",
    description:
      "Dry-run or apply safe coordination stream compatibility repairs, such as legacy-hash repair for context-bearing events.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_guard",
    description: "Check whether a lane may write the provided exact paths.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_report",
    description: "Append a generic coordination lifecycle report.",
    inputSchema: coordinationActionInputSchema(
      "report",
      "Optional dedicated-tool action marker.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_message",
    description: "Send generic coordination mail to a lane/address.",
    inputSchema: coordinationActionInputSchema(
      ["message", "send"],
      "Optional dedicated-tool action marker. Use coordination_mail for aggregate mail actions.",
    ),
  },
  {
    name: "ocentra_enforcer_coordination_workers",
    description: "Return compact worker status.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_coordination_tasks",
    description: "Return active task status.",
    inputSchema: coordinationInputSchema(),
  },
  {
    name: "ocentra_enforcer_explain",
    description:
      "Explain one Ocentra Enforcer rule ID and give the docs anchor/fix hint.",
    inputSchema: {
      type: "object",
      additionalProperties: false,
      required: ["ruleId"],
      properties: {
        ruleId: {
          type: "string",
          description: "Rule ID such as RR-7.3.",
        },
      },
    },
  },
];

const TOOLS = [
  ...CANONICAL_TOOLS,
  ...CANONICAL_TOOLS.map((tool) => ({
    ...tool,
    name: tool.name.replace("ocentra_enforcer_", "rust_rules_"),
    description: `Legacy alias for ${tool.name}; kept for one Rust-pack compatibility release.`,
  })),
];
const TOOL_SCHEMAS = new Map(TOOLS.map((tool) => [tool.name, tool.inputSchema]));

let inputBuffer = Buffer.alloc(0);
const validationHistory = new Map();

process.stdin.on("data", (chunk) => {
  inputBuffer = Buffer.concat([inputBuffer, chunk]);
  processInputFrames();
});

process.stdin.on("end", () => {
  process.exit(0);
});

function processInputFrames() {
  while (inputBuffer.length > 0) {
    const frame = readFrame();
    if (frame === null) return;
    if (frame.body.trim().length === 0) continue;
    handleRawMessage(frame.body, frame.framing);
  }
}

function readFrame() {
  const prefix = inputBuffer
    .slice(0, Math.min(inputBuffer.length, 64))
    .toString("utf8")
    .trimStart();
  if (!prefix.toLowerCase().startsWith("content-length:")) {
    const lineEnd = inputBuffer.indexOf("\n");
    if (lineEnd === -1) return null;
    const body = inputBuffer
      .slice(0, lineEnd)
      .toString("utf8")
      .replace(/\r$/u, "");
    inputBuffer = inputBuffer.slice(lineEnd + 1);
    return { body, framing: "ndjson" };
  }

  const crlfHeaderEnd = inputBuffer.indexOf("\r\n\r\n");
  const lfHeaderEnd = inputBuffer.indexOf("\n\n");
  let headerEnd = -1;
  let separatorLength = 0;
  if (
    crlfHeaderEnd !== -1 &&
    (lfHeaderEnd === -1 || crlfHeaderEnd < lfHeaderEnd)
  ) {
    headerEnd = crlfHeaderEnd;
    separatorLength = 4;
  } else if (lfHeaderEnd !== -1) {
    headerEnd = lfHeaderEnd;
    separatorLength = 2;
  }
  if (headerEnd === -1) return null;

  const header = inputBuffer.slice(0, headerEnd).toString("utf8");
  const lengthMatch = /content-length:\s*(\d+)/iu.exec(header);
  if (!lengthMatch) {
    throw new Error("MCP frame missing Content-Length header.");
  }

  const contentLength = Number(lengthMatch[1]);
  const messageStart = headerEnd + separatorLength;
  const messageEnd = messageStart + contentLength;
  if (inputBuffer.length < messageEnd) return null;

  const body = inputBuffer.slice(messageStart, messageEnd).toString("utf8");
  inputBuffer = inputBuffer.slice(messageEnd);
  return { body, framing: "content-length" };
}

function handleRawMessage(raw, framing) {
  let message;
  try {
    message = JSON.parse(raw);
  } catch (error) {
    sendError(null, -32700, `Parse error: ${error.message}`, framing);
    return;
  }

  Promise.resolve()
    .then(() => handleMessage(message, framing))
    .catch((error) => {
      if (message.id !== undefined) {
        sendError(
          message.id,
          -32603,
          error instanceof Error ? error.message : String(error),
          framing,
        );
      }
    });
}

async function handleMessage(message, framing) {
  if (
    message.id === undefined &&
    String(message.method ?? "").startsWith("notifications/")
  )
    return;

  switch (message.method) {
    case "initialize":
      sendResult(
        message.id,
        {
          protocolVersion:
            message.params?.protocolVersion ?? MCP_PROTOCOL_VERSION,
          capabilities: { tools: {} },
          serverInfo: {
            name: PACKAGE_JSON.name,
            version: PACKAGE_JSON.version,
          },
        },
        framing,
      );
      return;
    case "ping":
      sendResult(message.id, {}, framing);
      return;
    case "tools/list":
      sendResult(message.id, { tools: TOOLS }, framing);
      return;
    case "tools/call":
      sendResult(message.id, await callTool(message.params ?? {}), framing);
      return;
    case "resources/list":
      sendResult(message.id, { resources: [] }, framing);
      return;
    case "resources/templates/list":
      sendResult(message.id, { resourceTemplates: [] }, framing);
      return;
    case "prompts/list":
      sendResult(message.id, { prompts: [] }, framing);
      return;
    case "shutdown":
      sendResult(message.id, null, framing);
      return;
    default:
      sendError(
        message.id,
        -32601,
        `Unknown method: ${message.method}`,
        framing,
      );
  }
}

async function callTool(params) {
  const name = normalizeToolName(params.name);
  const args = params.arguments ?? {};
  try {
    rejectUnexpectedArguments(name, args);
    if (name === "ocentra_enforcer_mcp_status") {
      return toolJson(mcpStatus());
    }
    const freshness = mcpStatus();
    if (shouldBlockStaleMcpTool(name, args, freshness)) {
      return toolJson(mcpStaleError(name, freshness, args));
    }
    if (name === "ocentra_enforcer_route") {
      return {
        isError: false,
        content: [
          {
            type: "text",
            text: JSON.stringify(
              buildRouteReport(decodeRouteRequest(args), SERVER_ROOT),
              null,
              2,
            ),
          },
        ],
      };
    }
    if (name === "ocentra_enforcer_scan") {
      const decoded = decodeScanToolArguments(args);
      return runCli(decoded.cargo ? "cargo" : "scan", decoded);
    }
    if (name === "ocentra_enforcer_check") {
      return runCli("check", decodeCheckToolArguments(args));
    }
    if (name === "ocentra_enforcer_doctor") {
      return runCli("doctor", decodeDoctorToolArguments(args));
    }
    if (name === "ocentra_enforcer_coordination_health") {
      return toolJsonAsync(coordinationHealth(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_init") {
      return toolJsonAsync(coordinationInit(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_presence") {
      return toolJsonAsync(coordinationPresence(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_index") {
      return toolJsonAsync(coordinationIndex(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_streams") {
      return toolJsonAsync(coordinationStreams(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_sync") {
      return toolJsonAsync(coordinationSync(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_peer") {
      return toolJsonAsync(coordinationPeer(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_ensure") {
      return toolJsonAsync(coordinationEnsure(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_compact") {
      return toolJsonAsync(coordinationCompact(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_notify") {
      return toolJsonAsync(coordinationNotify(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_mail") {
      return toolJsonAsync(coordinationMail(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_status") {
      return toolJsonAsync(coordinationStatus(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_inbox") {
      return toolJsonAsync(coordinationInbox(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_claim") {
      return toolJsonAsync(coordinationClaim(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_release") {
      return toolJsonAsync(coordinationRelease(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_closeout") {
      return toolJsonAsync(coordinationCloseout(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_repair") {
      return toolJsonAsync(coordinationRepair(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_guard") {
      return toolJsonAsync(coordinationGuard(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_report") {
      return toolJsonAsync(coordinationReport(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_message") {
      return toolJsonAsync(coordinationMessage(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_workers") {
      return toolJsonAsync(coordinationWorkers(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_coordination_tasks") {
      return toolJsonAsync(coordinationTasks(decodeCoordinationToolArguments(args)));
    }
    if (name === "ocentra_enforcer_explain") {
      return runCli("explain", decodeExplainToolArguments(args));
    }
    if (name === "ocentra_enforcer_run") {
      return toolJson(runHarness(decodeRunToolArguments(args)));
    }
    if (name === "ocentra_enforcer_proof_route") {
      return toolJson(routeProofs(args, SERVER_ROOT));
    }
    if (name === "ocentra_enforcer_proof_run") {
      return toolJson(runProof(args, SERVER_ROOT));
    }
    if (name === "ocentra_enforcer_proof_status") {
      return toolJson(proofStatus(args));
    }
    if (name === "ocentra_enforcer_proof_inventory") {
      return toolJson(inventoryProofs(args));
    }
    if (name === "ocentra_enforcer_proof_import_legacy") {
      return toolJson(importLegacyProof(args, SERVER_ROOT));
    }
    if (name === "ocentra_enforcer_proof_parity") {
      return toolJson(proofParity(args));
    }
    if (name === "ocentra_enforcer_proof_claim") {
      return toolJson(claimProof(args, SERVER_ROOT));
    }
    if (name === "ocentra_enforcer_proof_last_failure") {
      return toolJson(proofLastFailure(args));
    }
    if (name === "ocentra_enforcer_proof_diagnostics") {
      return toolJson(proofDiagnostics(args));
    }
    if (name === "ocentra_enforcer_proof_artifact") {
      return toolJson(proofArtifact(args));
    }
    if (name === "ocentra_enforcer_proof_reset") {
      return toolJson(proofReset(args));
    }
    if (name === "ocentra_enforcer_proof_prune") {
      return toolJson(proofPrune(args));
    }
    if (name === "ocentra_enforcer_proof_export") {
      return toolJson(proofExport(args));
    }
    if (name === "ocentra_enforcer_run_status") {
      const decoded = decodeRunQueryArguments(args);
      const summary = runSummary(decoded);
      const validationSummary = latestValidationSummary(decoded);
      const artifact =
        summary && decoded.artifact ? readArtifact(decoded) : undefined;
      return toolJson({
        ok: true,
        summary: summary ?? validationSummary,
        summaryType: summary
          ? "harness"
          : validationSummary
            ? "validation"
            : "none",
        validationSummary,
        ...(artifact === undefined ? {} : { artifact }),
      });
    }
    if (name === "ocentra_enforcer_diagnostics") {
      return toolJson(runDiagnostics(decodeRunQueryArguments(args)));
    }
    if (name === "ocentra_enforcer_last_failure") {
      return toolJson(lastFailure(decodeRunQueryArguments(args)));
    }
    if (name === "ocentra_enforcer_artifact") {
      return toolJson(readArtifact(decodeRunQueryArguments(args)));
    }
    if (name === "ocentra_enforcer_prune_runs") {
      return toolJson(pruneRuns(decodeRunQueryArguments(args)));
    }
    if (name === "ocentra_enforcer_reset_runs") {
      return toolJson(resetRuns(decodeRunQueryArguments(args)));
    }
    return toolError(`Unknown tool: ${params.name}`);
  } catch (error) {
    return toolError(error instanceof Error ? error.message : String(error));
  }
}

function toolJsonAsync(promise) {
  return promise.then(toolJson, (error) =>
    toolError(error instanceof Error ? error.message : String(error)),
  );
}

function toolJson(value) {
  return {
    isError: value?.ok === false,
    content: [{ type: "text", text: JSON.stringify(value, null, 2) }],
  };
}

function mcpStatus() {
  const current = buildMcpFingerprint();
  const stale = STARTUP_FINGERPRINT.digest !== current.digest;
  const hashCompatibility = coordinationHashCompatibility();
  const hashCompatible = hashCompatibility.ok;
  const directWritesAllowed = !stale && hashCompatible;
  return {
    ok: directWritesAllowed,
    stale,
    reloadRequired: !directWritesAllowed,
    writeCompatible: directWritesAllowed,
    directWritesAllowed,
    hashCompatible,
    hashCompatibility,
    packRoot: SERVER_ROOT,
    processId: process.pid,
    startedAt: SERVER_STARTED_AT,
    nodeVersion: process.version,
    runningVersion: PACKAGE_JSON.version,
    currentVersion: current.packageVersion,
    startup: STARTUP_FINGERPRINT,
    current,
    changedFiles: changedFingerprintFiles(
      STARTUP_FINGERPRINT.files,
      current.files,
    ),
    nextStep: stale
      ? "Restart Codex Desktop/MCP or use ocentra_enforcer_run to invoke the updated CLI from the pack root."
      : hashCompatible
        ? "MCP server fingerprint matches the current Enforcer files and coordination hash compatibility is valid."
        : "Restart Codex Desktop/MCP or use ocentra_enforcer_run; coordination hash compatibility failed.",
  };
}

function shouldBlockStaleMcpTool(name, args, freshness) {
  if (freshness.directWritesAllowed === true) return false;
  if (COORDINATION_WRITE_TOOLS.has(name)) return true;
  if (name === "ocentra_enforcer_coordination_mail") {
    return ["send", "ack"].includes(String(args.action ?? "").toLowerCase());
  }
  if (name === "ocentra_enforcer_coordination_peer") {
    return ["add", "remove", "sync"].includes(
      String(args.action ?? "").toLowerCase(),
    );
  }
  if (name === "ocentra_enforcer_coordination_repair") {
    return args.write === true || args.dryRun === false;
  }
  return false;
}

function mcpStaleError(name, freshness, args = {}) {
  const reason = freshness.hashCompatible === false
    ? "coordination hash compatibility failed"
    : "MCP server is stale";
  const fallback = buildStaleFallback(name, args);
  return {
    ok: false,
    error: `${reason}; refusing ${name} because it may write incompatible coordination events.`,
    operation: name,
    directWritesAllowed: false,
    writeCapable: false,
    fallbackAvailable: fallback !== null,
    reloadRequired: true,
    fallback,
    nextStep: fallback
      ? `Restart Codex Desktop/MCP, or call ${fallback.recommendedTool} with fallback.enforcerRunArguments.`
      : "Restart Codex Desktop/MCP, or use ocentra_enforcer_run to invoke the updated CLI from the pack root.",
    mcpFreshness: freshness,
  };
}

function buildStaleFallback(name, args = {}) {
  const cliArgs = coordinationFallbackArgs(name, args);
  if (cliArgs.length === 0) return null;
  const command = [process.execPath, CLI_PATH, ...cliArgs];
  return {
    recommendedTool: "ocentra_enforcer_run",
    cwd: SERVER_ROOT,
    command,
    commandLine: command.map(quoteCommandArg).join(" "),
    enforcerRunArguments: {
      root: SERVER_ROOT,
      tool: "ocentra-enforcer-coordination-fallback",
      command,
    },
  };
}

function coordinationFallbackArgs(name, args) {
  const command = coordinationFallbackCommand(name, args);
  if (command === null) return [];
  return [
    "coordination",
    command,
    ...coordinationGlobalFallbackArgs(args),
    ...coordinationCommandFallbackArgs(command, args),
    "--json",
  ];
}

function coordinationFallbackCommand(name, args) {
  if (name === "ocentra_enforcer_coordination_init") return "init";
  if (name === "ocentra_enforcer_coordination_claim") return "claim";
  if (name === "ocentra_enforcer_coordination_release") return "release";
  if (name === "ocentra_enforcer_coordination_closeout") return "closeout";
  if (name === "ocentra_enforcer_coordination_report") return "report";
  if (name === "ocentra_enforcer_coordination_message") return "message";
  if (name === "ocentra_enforcer_coordination_sync") return "sync";
  if (name === "ocentra_enforcer_coordination_ensure") return "ensure";
  if (name === "ocentra_enforcer_coordination_compact") return "compact";
  if (name === "ocentra_enforcer_coordination_repair") return "repair";
  if (name === "ocentra_enforcer_coordination_mail") {
    const action = String(args.action ?? "").toLowerCase();
    if (action === "send") return "message";
    if (action === "ack") return "ack";
  }
  if (name === "ocentra_enforcer_coordination_peer") {
    const action = String(args.action ?? "").toLowerCase();
    if (["add", "remove", "sync"].includes(action)) return "peer";
  }
  return null;
}

function coordinationGlobalFallbackArgs(args) {
  const result = [];
  pushOption(result, "--state-root", args.stateRoot);
  pushOption(result, "--hub", args.hub);
  return result;
}

function coordinationCommandFallbackArgs(command, args) {
  if (command === "init") {
    return [
      ...(args.hub ? [String(args.hub)] : []),
      ...commonCoordinationOptions(args, { includePaths: false }),
    ];
  }
  if (command === "claim" || command === "release") {
    return [
      ...(args.lane ? [String(args.lane)] : []),
      ...stringArray(args.paths),
      ...commonCoordinationOptions(args, { includeLane: false, includePaths: false }),
    ];
  }
  if (command === "closeout") {
    const closeoutArgs = commonCoordinationOptions(args, { includePaths: false });
    pushOption(closeoutArgs, "--owner", args.owner);
    if (args.allOwned === true) closeoutArgs.push("--all-owned");
    if (args.allLanes === true) closeoutArgs.push("--all-lanes");
    if (args.allowOtherNode === true) closeoutArgs.push("--allow-other-node");
    if (args.releaseOwned === false) closeoutArgs.push("--no-release");
    if (args.repairStale === false) closeoutArgs.push("--no-repair-stale");
    return closeoutArgs;
  }
  if (command === "guard") {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...pathOption("--paths", args.paths ?? args.changedPaths),
    ];
  }
  if (command === "repair") {
    const repairArgs = [String(args.action ?? "legacy-hash")];
    repairArgs.push(...commonCoordinationOptions(args));
    pushOption(repairArgs, "--owner", args.owner);
    if (args.write === true) repairArgs.push("--write");
    if (args.dryRun === true) repairArgs.push("--dry-run");
    return repairArgs;
  }
  if (command === "message" || command === "msg") {
    const to = args.to ?? args.lane;
    const body = args.body ?? args.message ?? args.summary ?? args.subject;
    const messageArgs = commonCoordinationOptions(args, {
      includeLane: false,
      includePaths: false,
    });
    pushOption(messageArgs, "--from", args.from);
    pushOption(messageArgs, "--to", to);
    pushOption(messageArgs, "--subject", args.subject);
    pushOption(messageArgs, "--body", body);
    return messageArgs;
  }
  if (command === "ack") {
    return [
      ...commonCoordinationOptions(args, { includePaths: false }),
      ...(args.messageId ? [String(args.messageId)] : []),
      ...(args.id ? [String(args.id)] : []),
    ];
  }
  return commonCoordinationOptions(args);
}

function commonCoordinationOptions(args, options = {}) {
  const result = [];
  if (options.includeLane !== false) pushOption(result, "--lane", args.lane);
  pushOption(result, "--root", args.root);
  pushOption(result, "--repo-root", args.repoRoot);
  pushOption(result, "--worktree-root", args.worktreeRoot);
  pushOption(result, "--cwd", args.cwd);
  pushOption(result, "--project-id", args.projectId);
  pushOption(result, "--git-remote", args.gitRemote);
  pushOption(result, "--branch", args.branch);
  pushOption(result, "--commit", args.commit);
  pushOption(result, "--codex-thread-id", args.codexThreadId);
  pushOption(result, "--codex-session-id", args.codexSessionId);
  pushOption(result, "--session-id", args.sessionId);
  pushOption(result, "--operation", args.operation);
  pushOption(result, "--lock-kind", args.lockKind);
  pushOption(result, "--on-conflict", args.onConflict);
  pushOption(result, "--claim-group", args.claimGroup);
  pushOption(result, "--wait-ms", args.waitMs);
  pushOption(result, "--limit", args.limit);
  if (options.includePaths !== false) result.push(...pathOption("--paths", args.paths ?? args.changedPaths));
  result.push(...reasonOption(args));
  return result;
}

function reasonOption(args) {
  return args.reason ? ["--reason", String(args.reason)] : [];
}

function pathOption(name, value) {
  const paths = stringArray(value);
  return paths.length > 0 ? [name, paths.join(",")] : [];
}

function pushOption(result, name, value) {
  if (value !== undefined && value !== null && value !== "") {
    result.push(name, String(value));
  }
}

function stringArray(value) {
  if (Array.isArray(value)) return value.map(String).filter(Boolean);
  if (value === undefined || value === null || value === "") return [];
  return String(value)
    .split(/[,\n]/u)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function quoteCommandArg(value) {
  const text = String(value);
  if (/^[A-Za-z0-9_./:=+-]+$/u.test(text)) return text;
  return `"${text.replaceAll("\\", "\\\\").replaceAll('"', '\\"')}"`;
}

function buildMcpFingerprint() {
  const files = MCP_FINGERPRINT_FILES.map(fingerprintFile);
  const digest = createHash("sha256")
    .update(
      JSON.stringify(
        files.map((file) => ({
          path: file.path,
          exists: file.exists,
          sha256: file.sha256,
          byteLength: file.byteLength,
        })),
      ),
    )
    .digest("hex");
  return {
    digest,
    packageVersion: readPackageVersion(),
    files,
  };
}

function fingerprintFile(filePath) {
  const label = normalizeFingerprintLabel(filePath);
  const resolved = resolveFingerprintFile(filePath);
  if (!fs.existsSync(resolved)) {
    return {
      path: label,
      resolvedPath: resolved,
      exists: false,
      sha256: null,
      byteLength: 0,
      mtimeMs: null,
    };
  }
  const buffer = fs.readFileSync(resolved);
  const stat = fs.statSync(resolved);
  return {
    path: label,
    resolvedPath: resolved,
    exists: true,
    sha256: createHash("sha256").update(buffer).digest("hex"),
    byteLength: buffer.length,
    mtimeMs: stat.mtimeMs,
  };
}

function changedFingerprintFiles(startupFiles, currentFiles) {
  const startupByPath = new Map(startupFiles.map((file) => [file.path, file]));
  const currentByPath = new Map(currentFiles.map((file) => [file.path, file]));
  const paths = [...new Set([...startupByPath.keys(), ...currentByPath.keys()])].sort();
  return paths
    .map((filePath) => {
      const startup = startupByPath.get(filePath);
      const current = currentByPath.get(filePath);
      const changed =
        startup?.exists !== current?.exists ||
        startup?.sha256 !== current?.sha256 ||
        startup?.byteLength !== current?.byteLength;
      return changed
        ? {
            path: filePath,
            startup: summarizeFingerprintEntry(startup),
            current: summarizeFingerprintEntry(current),
          }
        : null;
    })
    .filter(Boolean);
}

function summarizeFingerprintEntry(entry) {
  return entry
    ? {
        exists: entry.exists,
        sha256: entry.sha256,
        byteLength: entry.byteLength,
      }
    : null;
}

function readPackageVersion() {
  try {
    return JSON.parse(
      fs.readFileSync(path.join(SERVER_ROOT, "package.json"), "utf8"),
    ).version;
  } catch {
    return null;
  }
}

function extraFingerprintFiles() {
  return String(process.env.OCENTRA_ENFORCER_MCP_FINGERPRINT_EXTRA ?? "")
    .split(path.delimiter)
    .map((entry) => entry.trim())
    .filter(Boolean);
}

function resolveFingerprintFile(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath)
    : path.join(SERVER_ROOT, filePath);
}

function normalizeFingerprintLabel(filePath) {
  return path.isAbsolute(filePath)
    ? path.resolve(filePath).replaceAll("\\", "/")
    : filePath.replaceAll("\\", "/");
}

function normalizeToolName(name) {
  return String(name ?? "").replace(/^rust_rules_/u, "ocentra_enforcer_");
}

function rejectUnexpectedArguments(name, args) {
  if (!args || typeof args !== "object" || Array.isArray(args)) return;
  const schema = TOOL_SCHEMAS.get(name);
  if (!schema || schema.additionalProperties !== false) return;
  const allowed = new Set(Object.keys(schema.properties ?? {}));
  const unexpected = Object.keys(args).filter((key) => !allowed.has(key));
  if (unexpected.length === 0) return;
  throw new Error(
    `${name} unexpected argument(s): ${unexpected.sort((left, right) => left.localeCompare(right)).join(", ")}`,
  );
}

function toolError(message) {
  const body = {
    ok: false,
    error: String(message),
    message: String(message),
  };
  return {
    isError: true,
    content: [{ type: "text", text: JSON.stringify(body, null, 2) }],
  };
}

function routeRules(args) {
  const root = path.resolve(args.root ?? process.cwd());
  const registry = loadRuleRegistry();
  const profileName = resolveProfileName(root, args);
  const explicitRuleId = args.ruleId?.toUpperCase() ?? null;
  const families = explicitRuleId ? [] : routeFamilies(args);
  const rules = explicitRuleId
    ? registry.rules.filter((rule) => rule.id === explicitRuleId)
    : registry.rules.filter((rule) => families.has(rule.family));
  const docs = uniqueSorted(rules.map((rule) => rule.doc));

  return {
    ok: true,
    productName: registry.productName,
    profileName,
    index: "rules/INDEX.md",
    scope: describeRouteScope(args),
    docs,
    rules: rules.map((rule) => ({
      id: rule.id,
      family: rule.family,
      severity: rule.severity,
      doc: rule.doc,
      validator: rule.validator,
    })),
  };
}

function loadRuleRegistry() {
  return decodeRuleRegistry(
    JSON.parse(fs.readFileSync(RULE_REGISTRY_PATH, "utf8")),
  );
}

function resolveProfileName(root, args) {
  if (args.configPath) {
    const configPath = path.isAbsolute(args.configPath)
      ? args.configPath
      : path.join(root, args.configPath);
    if (!fs.existsSync(configPath)) return "custom";
    const parsed = JSON.parse(fs.readFileSync(configPath, "utf8"));
    return parsed.profileName ?? "custom";
  }
  return args.profile ?? "strict";
}

function routeFamilies(args) {
  if (args.scope === "crate" || args.scope === "workspace") {
    return new Set([
      "source",
      "domain",
      "imports-modules",
      "async-runtime",
      "toolchain-cargo",
      "dependencies",
    ]);
  }

  const files = Array.isArray(args.files) ? args.files : [];
  const families = new Set();
  for (const file of files) {
    for (const family of routeFamiliesForFile(file)) families.add(family);
  }
  return families;
}

function routeFamiliesForFile(file) {
  const normalized = file.split(/[\\/]+/u).pop() ?? file;
  if (file.endsWith(".rs"))
    return ["source", "domain", "imports-modules", "async-runtime"];
  if (normalized === "Cargo.toml") return ["toolchain-cargo", "dependencies"];
  if (normalized === "Cargo.lock" || normalized === "deny.toml")
    return ["dependencies"];
  if (
    normalized === "rust-toolchain.toml" ||
    normalized === "clippy.toml" ||
    normalized === "rustfmt.toml"
  ) {
    return ["toolchain-cargo"];
  }
  return [];
}

function describeRouteScope(args) {
  if (args.ruleId) return { mode: "rule", ruleId: args.ruleId.toUpperCase() };
  if (args.scope === "crate")
    return { mode: "crate", crateName: args.crateName ?? null };
  if (args.scope === "diff")
    return {
      mode: "diff",
      base: args.base ?? null,
      head: args.head ?? null,
      files: args.files ?? [],
    };
  if (args.scope === "workspace") return { mode: "workspace" };
  return { mode: "files", files: args.files ?? [] };
}

function runCli(command, args) {
  if (command === "explain") {
    return runCliProcess(
      [CLI_PATH, "explain", args.ruleId, "--json"],
      process.cwd(),
      command,
      args,
    );
  }

  const root = path.resolve(args.root ?? process.cwd());
  const cliArgs = [CLI_PATH, command];
  if (command === "check") {
    cliArgs.push(args.check);
  }
  cliArgs.push("--root", root, "--json");
  const configPath = resolveConfigPath(root, args);
  if (configPath) {
    cliArgs.push("--config", configPath);
  }
  if (Array.isArray(args.languages) && args.languages.length > 0) {
    cliArgs.push("--languages", args.languages.join(","));
  }
  if (args.checkConfigPath) {
    cliArgs.push("--check-config", args.checkConfigPath);
  }
  if (args.output) {
    cliArgs.push("--output", args.output);
  }
  if (args.dryRun) {
    cliArgs.push("--dry-run");
  }
  if (args.staged) {
    cliArgs.push("--staged");
  }
  if (args.tracked) {
    cliArgs.push("--tracked");
  }
  if (args.strictEmptyTestTrees) {
    cliArgs.push("--strict-empty-test-trees");
  }
  cliArgs.push(...scopeArgs(args));

  return runCliProcess(cliArgs, root, command, args);
}

function runCliProcess(cliArgs, cwd, command = null, args = {}) {
  const result = spawnSync(process.execPath, cliArgs, {
    cwd,
    encoding: "utf8",
    shell: false,
  });

  const stdout = result.stdout?.trim() ?? "";
  const stderr = result.stderr?.trim() ?? "";
  const parsed = parseJson(stdout);
  const report =
    parsed && (command === "scan" || command === "cargo" || command === "check")
      ? maybeCompactReport(parsed, args)
      : parsed;
  if (
    parsed &&
    (command === "scan" || command === "cargo" || command === "check")
  ) {
    recordValidationReport(command, parsed, args);
  }
  const text =
    report != null
      ? JSON.stringify(report, null, 2)
      : stdout ||
        JSON.stringify({ ok: false, status: result.status, stderr }, null, 2);
  return {
    isError: (result.status ?? 1) !== 0,
    content: [
      {
        type: "text",
        text,
      },
    ],
  };
}

function parseJson(text) {
  if (!text || !text.trim().startsWith("{")) return null;
  try {
    return JSON.parse(text);
  } catch {
    return null;
  }
}

function maybeCompactReport(report, args) {
  const wantsCompact =
    args.summaryOnly === true ||
    args.diagnosticLimit !== undefined ||
    args.groupBy !== undefined ||
    args.includeScope === false;
  if (!wantsCompact) return report;

  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])];
  const limit = Math.max(
    0,
    Number.isFinite(args.diagnosticLimit)
      ? Math.trunc(args.diagnosticLimit)
      : 20,
  );
  const diagnostics = args.summaryOnly
    ? []
    : findings.slice(0, limit).map(compactFinding);
  const compact = {
    ok: report.ok,
    command: report.command,
    check: report.check,
    root: report.root,
    profileName: report.profileName,
    languages: report.languages,
    bySeverity: report.bySeverity ?? countBy(findings, "severity"),
    counts: {
      findings: findings.length,
      violations: report.violations?.length ?? 0,
      warnings: report.warnings?.length ?? 0,
      returned: diagnostics.length,
      truncated: findings.length > diagnostics.length,
    },
    ruleIds: uniqueSorted(findings.map((finding) => finding.ruleId)),
    docs: uniqueSorted(findings.map((finding) => finding.doc).filter(Boolean)),
    diagnostics,
  };
  if (args.groupBy) compact.groups = groupFindings(findings, args.groupBy);
  if (args.includeScope !== false) compact.scope = compactScope(report.scope);
  return compact;
}

function compactFinding(finding) {
  return {
    ruleId: finding.ruleId,
    severity: finding.severity ?? "error",
    file: finding.file,
    line: finding.line,
    detail: finding.detail,
    doc: finding.doc,
  };
}

function compactScope(scope) {
  if (!scope) return undefined;
  return {
    mode: scope.mode,
    fileCount: Array.isArray(scope.files) ? scope.files.length : undefined,
    sampleFiles: Array.isArray(scope.files)
      ? scope.files.slice(0, 20)
      : undefined,
    crateName: scope.crateName,
    base: scope.base,
    head: scope.head,
  };
}

function groupFindings(findings, mode) {
  const groups = new Map();
  for (const finding of findings) {
    const key = mode === "slice" ? sliceKey(finding.file) : finding.file;
    const group = groups.get(key) ?? {
      key,
      count: 0,
      bySeverity: {},
      ruleIds: new Set(),
      docs: new Set(),
      first: null,
    };
    group.count += 1;
    const severity = finding.severity ?? "error";
    group.bySeverity[severity] = (group.bySeverity[severity] ?? 0) + 1;
    group.ruleIds.add(finding.ruleId);
    if (finding.doc) group.docs.add(finding.doc);
    group.first ??= compactFinding(finding);
    groups.set(key, group);
  }
  return [...groups.values()]
    .map((group) => ({
      ...group,
      ruleIds: [...group.ruleIds].sort(),
      docs: [...group.docs].sort(),
    }))
    .sort((a, b) => b.count - a.count || a.key.localeCompare(b.key));
}

function sliceKey(file) {
  const parts = String(file ?? "").split("/");
  if (["apps", "packages", "crates", "tools"].includes(parts[0]) && parts[1])
    return `${parts[0]}/${parts[1]}`;
  return parts[0] || ".";
}

function recordValidationReport(command, report, args) {
  const root = path.resolve(args.root ?? report.root ?? process.cwd());
  const key = root.toLowerCase();
  const findings = [...(report.violations ?? []), ...(report.warnings ?? [])];
  const summary = {
    kind: command === "check" ? "check" : "scan",
    command: report.command,
    check: report.check,
    ok: report.ok,
    root,
    profileName: report.profileName,
    at: new Date().toISOString(),
    bySeverity: report.bySeverity ?? countBy(findings, "severity"),
    counts: {
      findings: findings.length,
      violations: report.violations?.length ?? 0,
      warnings: report.warnings?.length ?? 0,
    },
    ruleIds: uniqueSorted(findings.map((finding) => finding.ruleId)),
    docs: uniqueSorted(findings.map((finding) => finding.doc).filter(Boolean)),
    scope: compactScope(report.scope),
  };
  const entries = validationHistory.get(key) ?? [];
  entries.unshift(summary);
  validationHistory.set(key, entries.slice(0, 20));
}

function latestValidationSummary(args = {}) {
  const root = path.resolve(args.root ?? process.cwd()).toLowerCase();
  const entries = validationHistory.get(root) ?? [];
  if (args.tool === "check")
    return entries.find((entry) => entry.kind === "check") ?? null;
  if (args.tool === "scan")
    return entries.find((entry) => entry.kind === "scan") ?? null;
  return entries[0] ?? null;
}

function countBy(values, key) {
  const result = {};
  for (const value of values) {
    const group = value?.[key] ?? "unknown";
    result[group] = (result[group] ?? 0) + 1;
  }
  return result;
}

function resolveConfigPath(root, args) {
  if (args.configPath) {
    return path.isAbsolute(args.configPath)
      ? args.configPath
      : path.join(root, args.configPath);
  }

  const profile = args.profile ?? null;
  if (profile === null || profile === "" || profile === "strict") return null;
  if (!/^[A-Za-z0-9][A-Za-z0-9_.-]*$/u.test(profile)) {
    throw new Error(`Invalid profile name: ${profile}`);
  }

  const profilePath = path.join(SERVER_ROOT, "profiles", `${profile}.json`);
  if (!fs.existsSync(profilePath)) {
    throw new Error(
      `Unknown Ocentra Enforcer profile "${profile}". Expected ${profilePath}.`,
    );
  }
  return profilePath;
}

function uniqueSorted(values) {
  return [...new Set(values)].sort((a, b) => a.localeCompare(b));
}

function scopeArgs(args) {
  const inferredScope =
    args.scope ??
    (Array.isArray(args.files) && args.files.length > 0
      ? "files"
      : args.crateName
        ? "crate"
        : args.base || args.head
          ? "diff"
          : "workspace");

  if (inferredScope === "files") {
    if (!Array.isArray(args.files) || args.files.length === 0)
      throw new Error("files scope requires files.");
    return ["--files", ...args.files];
  }
  if (inferredScope === "crate") {
    if (!args.crateName) throw new Error("crate scope requires crateName.");
    return ["--crate", args.crateName];
  }
  if (inferredScope === "diff") {
    if (!args.base || !args.head)
      throw new Error("diff scope requires base and head.");
    return ["--base", args.base, "--head", args.head];
  }
  return ["--workspace"];
}

function sendResult(id, result, framing) {
  send({ jsonrpc: "2.0", id, result }, framing);
}

function sendError(id, code, message, framing) {
  send({ jsonrpc: "2.0", id, error: { code, message } }, framing);
}

function send(message, framing = "content-length") {
  const body = JSON.stringify(message);
  if (framing === "ndjson") {
    process.stdout.write(`${body}\n`);
  } else {
    process.stdout.write(
      `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`,
    );
  }
}
